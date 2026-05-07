# Symphonia 音频解码组件 Requirements

## What we need
将 Symphonia 纯 Rust 音频解码库封装为 WASI Component（wasm32-wasip2），提供 Resource 模式的 WIT 接口，支持全量解码和流式分块解码两种模式。

## Input & Output
**Input**: `list<u8>` — 音频文件原始字节（WAV/OGG/FLAC/MP3/MP4/MKV/AIFF/CAF 等容器格式）
**Output**: `audio-file` resource — 持有解码器状态，支持：
- `info()` → 音频元信息（codec/format/sample_rate/channels/duration/bit_depth）
- `decode-all()` → 全部 PCM f32 交错样本
- `decode-chunk(max_frames)` → 按帧数分块返回 PCM 样本
- `seek(timestamp_ms)` → 跳转到指定时间位置
- `position()` → 当前解码位置

## 支持的编解码器
| Codec | Feature Flag | 说明 |
|-------|-------------|------|
| FLAC | `flac` | 无损音频 |
| MP3 | `mp3` | MPEG-1/2 Audio Layer III |
| Vorbis | `vorbis` | OGG Vorbis |
| AAC | `aac` | AAC-LC |
| ALAC | `alac` | Apple Lossless |
| PCM | `pcm` | 脉冲编码调制 |
| ADPCM | `adpcm` | 自适应差分 PCM |

## 支持的容器格式
| Format | Feature Flag | 说明 |
|--------|-------------|------|
| WAV | `wav` | Waveform Audio |
| OGG | `ogg` | OGG 容器 |
| MKV/WebM | `mkv` | Matroska / WebM |
| ISO/MP4 | `isomp4` | MP4/M4A |
| AIFF | `aiff` | Audio Interchange File Format |
| CAF | `caf` | Core Audio Format |

## Success criteria
- [x] `cargo build --target wasm32-wasip2 --release` 编译成功
- [x] `cargo test` native 测试通过
- [x] WIT 接口包含 audio-file resource 及其所有方法
- [x] FLAC/MP3/Vorbis 至少三种格式解码正确
- [x] decode-all 返回完整 PCM f32 样本
- [x] decode-chunk 支持分块读取，可多次调用直到 EOF
- [x] seek 正确跳转位置，后续 decode-chunk 从新位置开始
- [x] info 返回正确的音频元信息
- [x] 无效音频数据返回错误而非 panic

## Edge cases
- **无效/损坏音频数据**: parse-audio 返回 Err，包含描述性错误信息
- **空输入**: parse-audio 返回 Err("empty input")
- **不支持的格式**: parse-audio 返回 Err("unsupported format: ...")
- **duration 未知**: audio-info.duration-ms 返回 None（如某些流式格式）
- **seek 超出范围**: seek 返回 Err("seek beyond end")
- **decode-chunk 到末尾**: 返回 Err("end of stream")
- **大文件（>100MB）**: 分块模式正常工作，不全量加载到内存
- **零声道/零采样率**: parse-audio 阶段拒绝
