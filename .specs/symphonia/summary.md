# Symphonia 音频解码组件 Summary

## Feature

将 [Symphonia](https://github.com/pdeljanov/Symphonia) 纯 Rust 音频解码库封装为 WASI Component，提供 Resource 模式的 WIT 接口，支持全量解码和流式分块解码。

## Metrics

| 指标 | 值 |
|------|-----|
| WASM 产物大小 | ~1.5 MB |
| Native 测试 | 18/18 pass |
| 支持 Codec | WAV(PCM), FLAC, MP3, OGG Vorbis, AIFF (+ AAC/ALAC/ADPCM/MKV/MP4/CAF 编译可用) |
| WIT 方法 | 5 (info, decode-all, decode-chunk, seek, position) |
| 实现行数 | ~500 lines (lib.rs) |
| 依赖 | symphonia 0.5 (MPL-2.0), wit-bindgen 0.57 (Apache-2.0) |
| 构建要求 | stable Rust, wasm32-wasip2 target |

## Files Created/Modified

| File | Action | Description |
|------|--------|-------------|
| `wit/audio-decoder.wit` | New | WIT 接口定义 (audio-info, audio-chunk, audio-file resource, parse-audio) |
| `crates/limpuai-wit-symphonia/Cargo.toml` | New | Crate 配置 |
| `crates/limpuai-wit-symphonia/src/lib.rs` | New | 完整实现 (~500 行) |
| `crates/limpuai-wit-symphonia/tests/fixtures/` | New | FLAC/OGG/MP3 测试 fixture |
| `Cargo.toml` | Modified | workspace members 新增 symphonia |
| `about.toml` | Modified | accepted 新增 MPL-2.0 |
| `THIRD-PARTY-LICENSES.md` | Modified | 重新生成，含 Symphonia MPL-2.0 许可证 |
| `README.md` | Modified | 全面重写，反映 4 组件架构 |

## Architecture

```
AudioFileImpl (struct)
  └── inner: RefCell<AudioFileInner>
        ├── format: Box<dyn FormatReader>    // Symphonia 容器读取器
        ├── decoder: Box<dyn Decoder>        // Symphonia 解码器
        ├── track_id, sample_rate, channels  // 音频元信息
        ├── format_name: String              // magic bytes 检测
        ├── duration_ms, bit_depth           // 可选元信息
        ├── position_ms: u64                 // 当前解码位置
        └── leftover: Vec<f32>               // 分块解码缓冲
```

## Key Decisions

| 决策 | 选择 | 理由 |
|------|------|------|
| 音频库 | Symphonia 0.5 (非 0.6-alpha) | crates.io 最新 stable，已验证 wasm32-wasip2 编译 |
| Resource 模式 | RefCell 封装在 AudioFileImpl 内部 | wit-bindgen 生成的 trait 要求 Self = AudioFileImpl |
| seek 返回类型 | `result<bool, string>` | WIT 不支持 `()` 作为 ok 类型 |
| format_name | magic bytes 检测 | Symphonia 0.5 FormatReader 无 format_info() (0.6 新增) |
| 分块缓冲 | `leftover: Vec<f32>` | 单 packet 可能包含超过 max_frames 的帧数 |

## Compliance

- ✅ `cargo build --target wasm32-wasip2 --release` 编译成功
- ✅ `cargo test` 18/18 native 测试通过
- ✅ WIT 接口包含 audio-file resource 及全部 5 个方法
- ✅ FLAC/MP3/Vorbis 三种格式解码正确（含 fixture 测试）
- ✅ decode-all 返回完整 PCM f32 样本
- ✅ decode-chunk 支持分块读取，可多次调用直到 EOF
- ✅ seek 正确跳转位置，后续 decode-chunk 从新位置开始
- ✅ info 返回正确的音频元信息
- ✅ 无效音频数据返回错误而非 panic (catch_unwind)

## cc-review Findings (All Resolved)

| # | Severity | Issue | Resolution |
|---|----------|-------|------------|
| 1 | P0 | 缺少 FLAC/MP3/Vorbis 测试 | 新增 tests/fixtures/ + 3 解码测试 + 4 格式检测测试 |
| 2 | P0 | design.md seek 签名未同步 | 更新为 `result<bool, string>` |
| 3 | P1 | format_name 硬编码 "auto" | 新增 `detect_format_name()` magic bytes 检测 |
| 4 | P1 | design.md 架构描述过时 | 全面更新 RefCell 位置、leftover 字段、format 检测流程 |

## Known Limitations

- format_name 仅支持 WAV/FLAC/OGG/MP3/AIFF 检测，其他格式返回 "unknown"
- seek 超出范围时返回 Symphonia 原始错误，非统一格式
- 未测试大文件 (>1MB) 分块解码
- Symphonia 0.5 — 升级到 0.6 后可改善 format_name 提取
