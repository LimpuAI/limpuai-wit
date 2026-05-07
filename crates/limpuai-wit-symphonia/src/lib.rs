wit_bindgen::generate!({
    world: "symphonia-audio",
    path: "../../wit",
});

use std::cell::RefCell;
use std::io::Cursor;

use exports::limpuai::data::audio_decoder::{
    AudioChunk, AudioInfo, Guest, GuestAudioFile,
};
use exports::limpuai::data::audio_decoder::AudioFile as AudioFileHandle;

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphError;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

// ---------------------------------------------------------------------------
// Internal state held by the audio-file resource.
// ---------------------------------------------------------------------------
struct AudioFileInner {
    format: Box<dyn symphonia::core::formats::FormatReader>,
    decoder: Box<dyn symphonia::core::codecs::Decoder>,
    track_id: u32,
    sample_rate: u32,
    channels: u16,
    codec: String,
    format_name: String,
    duration_ms: Option<u64>,
    bit_depth: Option<u16>,
    position_ms: u64,
    /// Buffered samples from previous decode_chunk that weren't consumed yet.
    leftover: Vec<f32>,
}

// ---------------------------------------------------------------------------
// WIT resource wrapper — uses RefCell because GuestAudioFile methods take &self.
// ---------------------------------------------------------------------------
struct AudioFileImpl {
    inner: RefCell<AudioFileInner>,
}

// ---------------------------------------------------------------------------
// Component — entry point for the WIT world.
// ---------------------------------------------------------------------------
struct Component;

// ---------------------------------------------------------------------------
// Detect container format from magic bytes.
// Symphonia 0.5 does not expose format name via the FormatReader trait (added in 0.6).
// ---------------------------------------------------------------------------
fn detect_format_name(data: &[u8]) -> String {
    if data.len() < 4 {
        return "unknown".to_string();
    }
    let magic = &data[0..4];
    match magic {
        b"RIFF" => "wav".to_string(),
        b"fLaC" => "flac".to_string(),
        b"OggS" => "ogg".to_string(),
        // MP3/MPEG Audio: sync word 0xFF combined with version/layer bits.
        // 0xFFE0 mask covers all valid MPEG Audio frame headers.
        _ if data[0] == 0xFF && data[1] & 0xE0 == 0xE0 => "mp3".to_string(),
        // AIFF
        b"FORM" if data.len() >= 12 && &data[8..12] == b"AIFF" => "aiff".to_string(),
        _ => "unknown".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Core parsing logic, shared between the Guest impl and tests.
// ---------------------------------------------------------------------------
fn parse_audio_inner(data: Vec<u8>) -> Result<AudioFileInner, String> {
    if data.is_empty() {
        return Err("empty input".to_string());
    }

    // Detect format from magic bytes before data is moved into Cursor.
    let format_name = detect_format_name(&data);

    let cursor = Cursor::new(data);
    let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

    let hint = Hint::new();
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("probe error: {e}"))?;

    // Extract track metadata while format is borrowed, then move format out.
    let (track_id, sample_rate, channels, n_frames, bit_depth, codec, decoder) = {
        let track = probed
            .format
            .default_track()
            .ok_or_else(|| "no audio track found".to_string())?;

        let id = track.id;
        let sr = track.codec_params.sample_rate.unwrap_or(0);
        let ch = track
            .codec_params
            .channels
            .map(|c: symphonia::core::audio::Channels| c.count() as u16)
            .unwrap_or(0);
        let nf = track.codec_params.n_frames;
        let bd = track.codec_params.bits_per_sample.map(|b| b as u16);
        let cs = format!("{:?}", track.codec_params.codec);

        let dec = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| format!("decoder create error: {e}"))?;

        (id, sr, ch, nf, bd, cs, dec)
    };

    if sample_rate == 0 || channels == 0 {
        return Err("invalid audio: zero sample rate or channels".to_string());
    }

    let duration_ms = n_frames.map(|f| f * 1000 / sample_rate as u64);

    Ok(AudioFileInner {
        format: probed.format,
        decoder,
        track_id,
        sample_rate,
        channels,
        codec,
        format_name,
        duration_ms,
        bit_depth,
        position_ms: 0,
        leftover: Vec::new(),
    })
}

// ---------------------------------------------------------------------------
// Guest trait — the parse-audio entry point.
// ---------------------------------------------------------------------------
impl Guest for Component {
    type AudioFile = AudioFileImpl;

    fn parse_audio(data: Vec<u8>) -> Result<AudioFileHandle, String> {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let inner = parse_audio_inner(data)?;
            Ok(AudioFileHandle::new(AudioFileImpl {
                inner: RefCell::new(inner),
            }))
        }))
        .map_err(|_| "audio parse panic: invalid data".to_string())?
    }
}

// ---------------------------------------------------------------------------
// GuestAudioFile — resource method implementations.
// ---------------------------------------------------------------------------
impl GuestAudioFile for AudioFileImpl {
    fn info(&self) -> AudioInfo {
        let inner = self.inner.borrow();
        AudioInfo {
            codec: inner.codec.clone(),
            format: inner.format_name.clone(),
            sample_rate: inner.sample_rate,
            channels: inner.channels,
            duration_ms: inner.duration_ms,
            bit_depth: inner.bit_depth,
        }
    }

    fn decode_all(&self) -> Result<Vec<f32>, String> {
        let mut inner = self.inner.borrow_mut();
        // Start with any leftover samples from previous decode_chunk calls.
        let mut all_samples = std::mem::take(&mut inner.leftover);

        loop {
            let packet = match inner.format.next_packet() {
                Ok(p) => p,
                Err(SymphError::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break
                }
                Err(SymphError::ResetRequired) => continue,
                Err(e) => return Err(format!("read error: {e}")),
            };

            if packet.track_id() != inner.track_id {
                continue;
            }

            let decoded = match inner.decoder.decode(&packet) {
                Ok(d) => d,
                Err(SymphError::DecodeError(_)) => continue,
                Err(e) => return Err(format!("decode error: {e}")),
            };

            let spec = *decoded.spec();
            let frames = decoded.capacity() as usize;
            let mut sample_buf = SampleBuffer::<f32>::new(frames as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);

            all_samples.extend_from_slice(sample_buf.samples());

            // Update position from packet timestamp (sample-rate time base).
            inner.position_ms =
                (packet.ts() as f64 * 1000.0 / inner.sample_rate as f64) as u64;
        }

        Ok(all_samples)
    }

    fn decode_chunk(&self, max_frames: u64) -> Result<AudioChunk, String> {
        let mut inner = self.inner.borrow_mut();
        let channels = inner.channels as usize;
        let max_samples = max_frames as usize * channels;
        let mut chunk_samples = std::mem::take(&mut inner.leftover);
        let mut first_ts_ms: Option<u64> = None;

        while chunk_samples.len() < max_samples {
            let packet = match inner.format.next_packet() {
                Ok(p) => p,
                Err(SymphError::IoError(ref e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break
                }
                Err(SymphError::ResetRequired) => continue,
                Err(e) => return Err(format!("read error: {e}")),
            };

            if packet.track_id() != inner.track_id {
                continue;
            }

            if first_ts_ms.is_none() {
                first_ts_ms =
                    Some((packet.ts() as f64 * 1000.0 / inner.sample_rate as f64) as u64);
            }

            let decoded = match inner.decoder.decode(&packet) {
                Ok(d) => d,
                Err(SymphError::DecodeError(_)) => continue,
                Err(e) => return Err(format!("decode error: {e}")),
            };

            let spec = *decoded.spec();
            let frames = decoded.capacity() as usize;
            let mut sample_buf = SampleBuffer::<f32>::new(frames as u64, spec);
            sample_buf.copy_interleaved_ref(decoded);

            let samples = sample_buf.samples();
            let remaining = max_samples - chunk_samples.len();
            let take = remaining.min(samples.len());
            chunk_samples.extend_from_slice(&samples[..take]);
            // Save excess samples as leftover for next call.
            if take < samples.len() {
                inner.leftover.extend_from_slice(&samples[take..]);
            }

            inner.position_ms =
                (packet.ts() as f64 * 1000.0 / inner.sample_rate as f64) as u64;
        }

        if chunk_samples.is_empty() {
            return Err("end of stream".to_string());
        }

        let frame_count = (chunk_samples.len() / channels) as u64;

        Ok(AudioChunk {
            samples: chunk_samples,
            frame_count,
            timestamp_ms: first_ts_ms.unwrap_or(0),
        })
    }

    fn seek(&self, timestamp_ms: u64) -> Result<bool, String> {
        let mut inner = self.inner.borrow_mut();
        // Discard buffered samples after seek.
        inner.leftover.clear();
        let seconds = timestamp_ms / 1000;
        let frac = (timestamp_ms % 1000) as f64 / 1000.0;
        let time = Time { seconds, frac };
        let seek_to = SeekTo::Time {
            time,
            track_id: Some(inner.track_id),
        };

        match inner.format.seek(SeekMode::Accurate, seek_to) {
            Ok(seeked) => {
                inner.position_ms =
                    (seeked.actual_ts as f64 * 1000.0 / inner.sample_rate as f64) as u64;
                Ok(true)
            }
            Err(e) => Err(format!("seek error: {e}")),
        }
    }

    fn position(&self) -> u64 {
        self.inner.borrow().position_ms
    }
}

export!(Component);

// ===========================================================================
// Tests — exercise internal logic directly (WIT handle needs component runtime).
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal valid 16-bit PCM WAV file from raw i16 samples.
    ///
    /// Samples must be interleaved: [L0, R0, L1, R1, …] for stereo.
    fn make_wav(samples: &[i16], channels: u16, sample_rate: u32) -> Vec<u8> {
        let bits_per_sample: u16 = 16;
        let byte_rate = sample_rate * channels as u32 * (bits_per_sample / 8) as u32;
        let block_align = channels * (bits_per_sample / 8);
        let data_size = (samples.len() * 2) as u32;
        let file_size_minus_8 = 36 + data_size;

        let mut buf = Vec::with_capacity(44 + data_size as usize);

        // RIFF header
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&file_size_minus_8.to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        // fmt sub-chunk
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes()); // sub-chunk size
        buf.extend_from_slice(&1u16.to_le_bytes()); // PCM
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&byte_rate.to_le_bytes());
        buf.extend_from_slice(&block_align.to_le_bytes());
        buf.extend_from_slice(&bits_per_sample.to_le_bytes());
        // data sub-chunk
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&data_size.to_le_bytes());
        // PCM samples
        for &s in samples {
            buf.extend_from_slice(&s.to_le_bytes());
        }
        buf
    }

    /// 2-channel, 44100 Hz, 4 frames (8 interleaved i16 samples).
    fn make_test_wav() -> Vec<u8> {
        let samples: &[i16] = &[1000, -1000, 2000, -2000, 3000, -3000, 4000, -4000];
        make_wav(samples, 2, 44100)
    }

    /// Parse bytes into an AudioFileImpl for testing (bypasses WIT handle).
    fn parse_for_test(data: &[u8]) -> Result<AudioFileImpl, String> {
        let inner = parse_audio_inner(data.to_vec())?;
        Ok(AudioFileImpl {
            inner: RefCell::new(inner),
        })
    }

    // ── Parsing ──────────────────────────────────────────────────────
    #[test]
    fn parse_wav() {
        let wav = make_test_wav();
        let af = parse_for_test(&wav).expect("should parse WAV");
        let info = af.info();
        assert_eq!(info.channels, 2);
        assert_eq!(info.sample_rate, 44100);
    }

    #[test]
    fn parse_empty() {
        let result = parse_audio_inner(Vec::new());
        assert!(result.is_err());
        let msg = result.err().unwrap();
        assert!(msg.contains("empty"));
    }

    #[test]
    fn parse_invalid() {
        let result = parse_audio_inner(vec![0xDE, 0xAD, 0xBE, 0xEF]);
        assert!(result.is_err());
    }

    // ── Info ─────────────────────────────────────────────────────────
    #[test]
    fn info_metadata() {
        let wav = make_test_wav();
        let af = parse_for_test(&wav).expect("parse");
        let info = af.info();

        assert_eq!(info.sample_rate, 44100);
        assert_eq!(info.channels, 2);
        // Duration: 4 frames / 44100 Hz ≈ 0.09 ms → rounds to 0
        assert_eq!(info.duration_ms, Some(0));
        // 16-bit PCM
        assert_eq!(info.bit_depth, Some(16));
    }

    // ── Decode All ───────────────────────────────────────────────────
    #[test]
    fn decode_all_samples() {
        let wav = make_test_wav();
        let af = parse_for_test(&wav).expect("parse");
        let samples = af.decode_all().expect("decode_all");

        // 4 frames × 2 channels = 8 f32 samples
        assert_eq!(samples.len(), 8);
        // First sample should be ≈ 1000 / 32767 ≈ 0.0305
        assert!((samples[0].abs() - 0.0305).abs() < 0.002);
        // Second sample should be ≈ -1000 / 32767 ≈ -0.0305
        assert!((samples[1].abs() - 0.0305).abs() < 0.002);
    }

    #[test]
    fn decode_all_updates_position() {
        let wav = make_test_wav();
        let af = parse_for_test(&wav).expect("parse");
        assert_eq!(af.position(), 0);

        let _ = af.decode_all().expect("decode_all");
        // After decoding all 4 frames at 44100 Hz, position ≈ 0 ms (very short)
        // Just verify it's non-negative and didn't panic.
        assert!(af.position() < 1000);
    }

    // ── Decode Chunk ─────────────────────────────────────────────────
    #[test]
    fn decode_chunk_basic() {
        let wav = make_test_wav();
        let af = parse_for_test(&wav).expect("parse");

        // Request 2 frames (4 f32 samples for stereo)
        let chunk = af.decode_chunk(2).expect("chunk 1");
        assert_eq!(chunk.frame_count, 2);
        assert_eq!(chunk.samples.len(), 4); // 2 frames × 2 channels
        assert_eq!(chunk.timestamp_ms, 0); // starts at beginning

        // Request next 2 frames
        let chunk2 = af.decode_chunk(2).expect("chunk 2");
        assert_eq!(chunk2.frame_count, 2);
        assert_eq!(chunk2.samples.len(), 4);
    }

    #[test]
    fn decode_chunk_eof() {
        let wav = make_test_wav();
        let af = parse_for_test(&wav).expect("parse");

        // Decode all in one big chunk
        let _ = af.decode_chunk(100).expect("chunk big");
        // Second call should hit EOF
        let result = af.decode_chunk(100);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("end of stream"));
    }

    #[test]
    fn decode_chunk_large_request() {
        let wav = make_test_wav();
        let af = parse_for_test(&wav).expect("parse");

        // Request more frames than available — should return what's there
        let chunk = af.decode_chunk(1000).expect("chunk oversized");
        assert_eq!(chunk.frame_count, 4);
        assert_eq!(chunk.samples.len(), 8);
    }

    // ── Seek ─────────────────────────────────────────────────────────
    #[test]
    fn seek_to_beginning() {
        let wav = make_test_wav();
        let af = parse_for_test(&wav).expect("parse");

        // Decode some first
        let _ = af.decode_chunk(2).expect("chunk");
        // Seek back to start
        af.seek(0).expect("seek");
        assert_eq!(af.position(), 0);

        // Should be able to decode again from the beginning
        let chunk = af.decode_chunk(2).expect("chunk after seek");
        assert_eq!(chunk.frame_count, 2);
        // First sample should match the original first sample
        assert!((chunk.samples[0].abs() - 0.0305).abs() < 0.002);
    }

    // ── Format detection ────────────────────────────────────────────
    #[test]
    fn format_detection_wav() {
        let wav = make_test_wav();
        let af = parse_for_test(&wav).expect("parse");
        assert_eq!(af.info().format, "wav");
    }

    #[test]
    fn format_detection_flac_magic() {
        // Minimal bytes starting with "fLaC" magic — enough to test detection.
        assert_eq!(detect_format_name(b"fLaC\x00\x00\x00"), "flac");
    }

    #[test]
    fn format_detection_ogg_magic() {
        assert_eq!(detect_format_name(b"OggS\x00\x00\x00"), "ogg");
    }

    #[test]
    fn format_detection_mp3_magic() {
        // MP3 sync word: 0xFF followed by byte with bits 7-5 set (0xE0 mask).
        assert_eq!(detect_format_name(b"\xFF\xFB\x90\x00"), "mp3");
    }

    // ── Multi-format decoding ──────────────────────────────────────
    // Test audio files in tests/fixtures/ generated by libsndfile.
    // Each contains ~100 frames of mono 44100 Hz sine wave.

    fn load_fixture(name: &str) -> Vec<u8> {
        let path = format!("tests/fixtures/{name}");
        std::fs::read(&path).unwrap_or_else(|e| panic!("failed to read fixture {path}: {e}"))
    }

    #[test]
    fn decode_flac() {
        let flac = load_fixture("sine_flac.flac");
        let af = parse_for_test(&flac).expect("should parse FLAC");
        let info = af.info();
        assert_eq!(info.format, "flac");
        assert_eq!(info.sample_rate, 44100);
        assert_eq!(info.channels, 1);
        assert_eq!(info.bit_depth, Some(16));

        let samples = af.decode_all().expect("should decode FLAC");
        assert!(!samples.is_empty(), "FLAC should produce samples");
    }

    #[test]
    fn decode_ogg_vorbis() {
        let ogg = load_fixture("sine_ogg.ogg");
        let af = parse_for_test(&ogg).expect("should parse OGG");
        let info = af.info();
        assert_eq!(info.format, "ogg");
        assert_eq!(info.sample_rate, 44100);
        assert_eq!(info.channels, 1);

        let samples = af.decode_all().expect("should decode OGG Vorbis");
        assert!(!samples.is_empty(), "OGG should produce samples");
    }

    #[test]
    fn decode_mp3() {
        let mp3 = load_fixture("sine_mp3.mp3");
        let af = parse_for_test(&mp3).expect("should parse MP3");
        let info = af.info();
        assert_eq!(info.format, "mp3");
        assert_eq!(info.sample_rate, 44100);
        assert_eq!(info.channels, 1);

        let samples = af.decode_all().expect("should decode MP3");
        assert!(!samples.is_empty(), "MP3 should produce samples");
    }

    // ── WAV generation helper ────────────────────────────────────────
    #[test]
    fn wav_helper_is_valid() {
        let wav = make_test_wav();
        // Verify RIFF header
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[12..16], b"fmt ");
        assert_eq!(&wav[36..40], b"data");
        // Verify it can be parsed by Symphonia
        let result = parse_for_test(&wav);
        assert!(result.is_ok());
    }
}
