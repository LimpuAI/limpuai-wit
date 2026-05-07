# Symphonia 音频解码组件 Design

## API contracts

**WIT 接口**: `limpuai:data/audio-decoder`

```
parse-audio: func(data: list<u8>) -> result<audio-file, string>

resource audio-file {
    info: func() -> audio-info;
    decode-all: func() -> result<list<f32>, string>;
    decode-chunk: func(max-frames: u64) -> result<audio-chunk, string>;
    seek: func(timestamp-ms: u64) -> result<bool, string>;
    position: func() -> u64;
}
```

**audio-info record**:
```
record audio-info {
    codec: string,
    format: string,
    sample-rate: u32,
    channels: u16,
    duration-ms: option<u64>,
    bit-depth: option<u16>,
}
```

**audio-chunk record**:
```
record audio-chunk {
    samples: list<f32>,      // interleaved PCM
    frame-count: u64,
    timestamp-ms: u64,
}
```

**World**: `symphonia-audio`

## Key decisions

- **Tech choice**: Symphonia 0.6 (workspace) — 纯 Rust 音频解码，`#![forbid(unsafe_code)]`，已验证编译到 wasm32-wasip2
- **Resource 模式**: 类似 polars-dataframe，audio-file 持有 Symphonia FormatReader + Decoder 状态，通过 RefCell 实现内部可变
- **PCM f32 输出**: 使用 Symphonia 的 `SampleBuffer<f32>` 将所有解码样本转换为交错 f32
- **分块解码**: decode-chunk 从解码器读取 max_frames 帧，返回交错 PCM + 时间戳。调用方循环调用直到 EOF
- **Seek**: 使用 Symphonia FormatReader 的 `seek()` 方法，传入 SeekMode::Accurate

## Crate 结构

```
crates/limpuai-wit-symphonia/
├── Cargo.toml
└── src/
    └── lib.rs
```

### Cargo.toml

```toml
[package]
name = "limpuai-wit-symphonia"
version.workspace = true
edition.workspace = true
license.workspace = true

[lib]
crate-type = ["cdylib"]

[dependencies]
symphonia = { version = "0.5", features = ["all"] }
wit-bindgen = "0.57"
```

> 注意：使用 symphonia 0.5 (crates.io 最新发布版) 而非 0.6-alpha。如需 0.6 新特性可后续升级。

### lib.rs 核心结构

> **注意**: 实际实现中，RefCell 封装在 `AudioFileImpl` 内部而非外部。
> Symphonia 0.5 不提供 `FormatReader::format_info()` 方法（0.6 新增），
> 因此使用 `detect_format_name()` 从输入数据 magic bytes 检测格式名。

```rust
wit_bindgen::generate!({
    world: "symphonia-audio",
    path: "../../wit",
});

use std::cell::RefCell;
use std::io::Cursor;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::io::MediaSourceStream;

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

struct AudioFileImpl {
    inner: RefCell<AudioFileInner>,
}

struct Component;

impl Guest for Component {
    type AudioFile = AudioFileImpl;

    fn parse_audio(data: Vec<u8>) -> Result<AudioFileHandle, String> {
        // 1. Detect format name from magic bytes (before data is moved)
        // 2. Create MediaSourceStream from Cursor
        // 3. Probe format
        // 4. Select default track
        // 5. Create decoder
        // 6. Extract metadata
        // 7. Return AudioFileImpl { inner: RefCell::new(inner) }
        // Wrapped in catch_unwind for safety
    }
}

impl GuestAudioFile for AudioFileImpl {
    fn info(&self) -> AudioInfo { ... }
    fn decode_all(&self) -> Result<Vec<f32>, String> { ... }
    fn decode_chunk(&self, max_frames: u64) -> Result<AudioChunk, String> { ... }
    fn seek(&self, timestamp_ms: u64) -> Result<bool, String> { ... }
    fn position(&self) -> u64 { ... }
}

export!(Component);
```

## 数据流

```
parse_audio(bytes)
   │
   ├── detect_format_name(&bytes) → "wav"/"flac"/"ogg"/"mp3" (magic bytes)
   ├── Cursor<Vec<u8>> → MediaSourceStream
   ├── get_probe().format() → FormatReader
   ├── get_codecs().make() → Decoder
   └── store in AudioFileImpl { inner: RefCell<AudioFileInner> }

decode_all()
   │
   ├── consume leftover buffer from previous decode_chunk calls
   ├── loop { format.next_packet() → decoder.decode() }
   ├── SampleBuffer<f32>::copy_interleaved_ref()
   └── collect all samples into Vec<f32>

decode_chunk(max_frames)
   │
   ├── consume leftover buffer first
   ├── loop until collected max_frames or EOF
   ├── SampleBuffer<f32>::copy_interleaved_ref()
   ├── buffer excess samples as leftover for next call
   └── return AudioChunk { samples, frame_count, timestamp_ms }

seek(timestamp_ms)
   │
   ├── clear leftover buffer
   ├── SeekTo::Time { time, track_id }
   └── format.seek(SeekMode::Accurate, seek_to)
```

## 类型映射

| Symphonia 类型 | WIT 类型 |
|---------------|---------|
| `AudioBufferRef` | 内部处理，不直接暴露 |
| `SampleBuffer<f32>` | `list<f32>` (交错 PCM) |
| `SignalSpec` (rate, channels) | `audio-info.sample-rate`, `audio-info.channels` |
| `CodecDescriptor` | `audio-info.codec` (string) |
| `FormatReader` trait | 内部持有，不暴露 |
| `Time` (seconds + frac) | `timestamp-ms: u64` (毫秒) |

## 依赖

| 库 | 版本 | 用途 |
|---|---|---|
| `symphonia` | 0.5 | 音频解码核心 |
| `wit-bindgen` | 0.57 | WIT 绑定生成 |

## Integration points
- 加入 workspace Cargo.toml members
- WIT 文件加入 wit/ 目录
- 编译命令：`cargo build --target wasm32-wasip2 --release -p limpuai-wit-symphonia`
