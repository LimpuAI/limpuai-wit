# Symphonia 音频解码组件 Tasks

## Progress
Goal: 将 Symphonia 封装为 WASI Component，支持 Resource 模式的音频解码
Status: 8/8 (100%)
Current: Done
Next: All tasks complete

## Tasks
- [x] 1. 创建 WIT 接口文件 `wit/audio-decoder.wit` — ref: requirements 全部, design API contracts
- [x] 2. 创建 crate 目录和 `crates/limpuai-wit-symphonia/Cargo.toml` — ref: design Crate 结构
- [x] 3. 实现 `parse_audio` 入口函数 — ref: requirements Input & Output, design 数据流
- [x] 4. 实现 `info()` 和 `position()` 方法 — ref: design 类型映射
- [x] 5. 实现 `decode_all()` 全量解码 — ref: requirements Success criteria #5, design 数据流
- [x] 6. 实现 `decode_chunk()` 分块解码 — ref: requirements Success criteria #6, design 数据流
- [x] 7. 实现 `seek()` 跳转 — ref: requirements Success criteria #7, design 数据流
- [x] 8. 更新 workspace Cargo.toml + 测试编译 + native 测试 — ref: requirements Success criteria #1-4

## Notes
- Symphonia 0.5 使用 `symphonia::default::get_probe()` 和 `symphonia::default::get_codecs()` 作为便捷入口
- `SampleBuffer<f32>` 需要在首次解码时根据 `AudioBufferRef` 的 spec 初始化
- seek 需要使用 `symphonia::core::formats::SeekTo::Time { time, track_id }`
- RefCell 模式参考 polars 组件的 DataframeImpl 实现
- wasm32-wasip2 下 SIMD 不可用，rustfft 使用纯 Rust fallback
