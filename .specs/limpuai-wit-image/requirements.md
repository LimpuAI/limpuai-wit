# limpuai-wit-image Requirements

## What we need

将 Rust 图像库 `image-rs/image` (0.25.x) 编译为 WASI Component (wasm32-wasip2)，提供图像解码、基本操作和编码能力。

宿主通过 WIT Resource 接口持有图像状态，支持：
- 解码多种图像格式（PNG/JPEG/GIF/WebP/TIFF/BMP/ICO/PNM/QOI/TGA/HDR/FF/EXR）
- 基本图像操作（resize/crop/blur/fliph/flipv/rotate90/rotate180/rotate270/brighten/contrast/grayscale/invert）
- 编码输出为多种格式（PNG/JPEG/GIF/WebP/TIFF/BMP/ICO/PNM）

不对 image-rs/image 源码做任何改动，仅做接口定义和编译导出。

## Input & Output

**Input**: `list<u8>` — 原始图像文件字节
**Output**: 通过 Resource 持有图像状态，可获取元数据、像素数据、执行操作、编码输出

## 成功标准

- [ ] `cargo build --target wasm32-wasip2 --release -p limpuai-wit-image` 编译成功
- [ ] 所有默认图像格式解码正确（PNG/JPEG/GIF/WebP/TIFF/BMP）
- [ ] Resource 接口支持基本操作（resize/crop/blur/flip/rotate/brighten/contrast/grayscale）
- [ ] 编码输出至少支持 PNG/JPEG/BMP/WebP/TIFF/GIF
- [ ] WASM 产物 < 5 MB
- [ ] Native target 测试通过（`cargo test -p limpuai-wit-image`）
- [ ] 无 rayon 依赖（WASM 单线程安全）

## 边界情况

- **空输入**: 返回明确错误信息
- **无效图像数据**: 返回解码错误，不 panic
- **超大图像**: 通过图像尺寸限制保护内存（WASM 线性内存有限）
- **不支持的格式**: 返回格式不支持错误
- **编码 JPEG 质量**: 默认质量 75，可选 1-100
- **Alpha 通道处理**: RGBA → RGB 编码时丢弃或保留 alpha

## 约束

- `image` crate 使用 `default-features = false`，不启用 rayon/nasm/avif-native
- WIT 接口只接受原始字节（`list<u8>`），不使用 `image::open()` 或 `image::save()`（避免 std::fs）
- 操作不改变原始图像（返回新的 resource handle）
- 复用项目已有的 `wit-bindgen` 0.57 版本

## 参考信息

- 现有组件模式: `limpuai-wit-symphonia`（Resource 模式）、`limpuai-wit-polars`（Resource + 链式操作）
- 底层库: `image` 0.25.x (MIT OR Apache-2.0)
- 所有编解码器均为纯 Rust，无系统依赖
