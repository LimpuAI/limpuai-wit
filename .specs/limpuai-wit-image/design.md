# limpuai-wit-image Design

## WIT 接口定义

**文件**: `wit/image-processor.wit`
**Interface**: `image-processor`
**World**: `image-data`（遵循项目惯例：world 名 ≠ interface 名，如 symphonia 使用 `world symphonia-audio` + `interface audio-decoder`）

```wit
package limpuai:data;

interface image-processor {
    enum color-type {
        l8,       // Grayscale 8-bit
        la8,      // Grayscale + Alpha 8-bit
        rgb8,     // RGB 8-bit
        rgba8,    // RGBA 8-bit
        l16,      // Grayscale 16-bit
        la16,     // Grayscale + Alpha 16-bit
        rgb16,    // RGB 16-bit
        rgba16,   // RGBA 16-bit
        rgb32f,   // RGB 32-bit float
        rgba32f,  // RGBA 32-bit float
    }

    enum output-format {
        png,
        jpeg,
        bmp,
        gif,
        tiff,
        ico,
        webp,
        pnm,
    }

    record image-meta {
        width: u32,
        height: u32,
        color-type: color-type,
        format: string,
    }

    resource image-handle {
        width: func() -> u32;
        height: func() -> u32;
        color-type: func() -> color-type;
        pixel-data: func() -> list<u8>;
        resize: func(width: u32, height: u32) -> result<image-handle, string>;
        crop: func(x: u32, y: u32, width: u32, height: u32) -> result<image-handle, string>;
        blur: func(sigma: f32) -> result<image-handle, string>;
        brighten: func(value: f32) -> result<image-handle, string>;
        contrast: func(value: f32) -> result<image-handle, string>;
        grayscale: func() -> result<image-handle, string>;
        invert: func() -> result<image-handle, string>;
        fliph: func() -> result<image-handle, string>;
        flipv: func() -> result<image-handle, string>;
        rotate90: func() -> result<image-handle, string>;
        rotate180: func() -> result<image-handle, string>;
        rotate270: func() -> result<image-handle, string>;
    }

    decode-image: func(data: list<u8>) -> result<image-handle, string>;
    encode-image: func(img: image-handle, format: output-format, quality: option<u8>) -> result<list<u8>, string>;
    image-info: func(data: list<u8>) -> result<image-meta, string>;
}

world image-data {
    export image-processor;
}
```

## API 合约

### decode-image
**Input**: `list<u8>` (原始图像字节)
**Output**: `image-handle` resource
**Errors**: 空输入 / 不支持格式 / 损坏数据

### image-info
**Input**: `list<u8>` (原始图像字节)
**Output**: `image-meta` (仅元数据，不解码像素)
**Errors**: 空输入 / 不支持格式

### encode-image
**Input**: `image-handle` (by-value，**handle 被 consume，调用后不可再用**) + `output-format` + `quality` (仅 JPEG 使用，1-100，默认 75)
**Output**: `list<u8>` (编码后的图像字节)
**Errors**: 不支持输出格式 / 编码失败
**注意**: 由于 WIT 语义中 resource by-value 参数会转移所有权，调用 `encode-image` 后该 handle 在宿主端失效。如需多次编码不同格式，需多次 decode 或使用 `pixel-data()` 自行处理。

### image-handle 操作
所有操作（`resize`/`crop`/`blur`/`brighten`/`contrast`/`grayscale`/`fliph`/`flipv`/`rotate90`/`rotate180`/`rotate270`）返回**新的** `image-handle`（不可变语义），原图不受影响。

`invert` 特殊：**原地修改原图像素**，同时返回一个新 handle 包装修改后的图像（克隆）。调用后原图 handle 和返回 handle 均持有反转后的像素数据。这是因为 WIT 签名要求返回 `result<image-handle, string>`，而设计意图是修改原图。

## 关键设计决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 接口模式 | Resource | 图像解码一次，多次操作/编码，避免重复解码 |
| 操作语义 | 不可变（返回新 handle） | 安全、可组合，宿主可链式操作 |
| 像素输出 | `list<u8>` 原始字节 | 简单直接，宿主按 color-type 解释 |
| rayon | 禁用 | WASM 单线程，rayon 无意义且增加体积 |
| 图像尺寸限制 | 无显式限制 | 依赖宿主控制 WASM 线性内存上限 |
| 16-bit/32f 像素 | 保留原始精度 | 按实际 color-type 输出，不做降级 |

## 数据流

```
[宿主] ──── list<u8> ───► [decode-image] ───► image-handle (Resource)
                                                      │
                                          ┌───────────┼───────────┐
                                          ▼           ▼           ▼
                                      [width/height] [pixel-data] [resize/blur/...]
                                                                          │
                                                                          ▼
                                                              image-handle (新)
                                                                          │
                                                                          ▼
                                                              [encode-image] ───► list<u8>
```

## 类型映射

### image-rs ColorType → WIT color-type

| image-rs ColorType | WIT color-type | 字节/像素 |
|---|---|---|
| L8 | l8 | 1 |
| La8 | la8 | 2 |
| Rgb8 | rgb8 | 3 |
| Rgba8 | rgba8 | 4 |
| L16 | l16 | 2 |
| La16 | la16 | 4 |
| Rgb16 | rgb16 | 6 |
| Rgba16 | rgba16 | 8 |
| Rgb32F | rgb32f | 12 |
| Rgba32F | rgba32f | 16 |

### image-rs ImageFormat → 检测字符串

| 格式 | 检测方式 |
|------|---------|
| PNG | magic `\x89PNG` |
| JPEG | magic `\xFF\xD8\xFF` |
| GIF | magic `GIF87a`/`GIF89a` |
| WebP | magic `RIFF....WEBP` |
| TIFF | magic `II`/`MM` |
| BMP | magic `BM` |
| 其他 | 通过 image-rs 内部检测 |

## 依赖配置

```toml
[dependencies]
image = { version = "0.25", default-features = false, features = [
    "bmp", "exr", "ff", "gif", "hdr", "ico", "jpeg", "png",
    "pnm", "qoi", "tga", "tiff", "webp"
    # 不含 rayon, nasm, avif-native, avif (ravif 较大)
] }
wit-bindgen = "0.57"
```

注意：`avif` 格式依赖 `ravif` crate，体积较大且编码慢，暂不启用。如需支持可后续添加。

## 集成点

- **workspace Cargo.toml**: 添加 `"crates/limpuai-wit-image"` 到 members
- **wit/ 目录**: 新增 `image-processor.wit`
- **THIRD-PARTY-LICENSES.md**: 运行 `cargo about` 更新
- **README.md**: 添加组件到一览表

## 与现有组件的关系

- 独立 world，不依赖 `types.wit` 中的 DataTable 类型
- 图像数据用 `list<u8>` 原始字节表示，不适合 DataTable 列式格式
- Resource 模式参考 `limpuai-wit-symphonia` 的 AudioFile 实现
