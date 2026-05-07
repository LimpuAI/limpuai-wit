# limpuai-wit-image Tasks

## Progress
Goal: 将 image-rs/image 编译为 WASI Component，提供图像解码/操作/编码能力
Status: 8/8 (100%)
Current: All tasks complete
Next: spec-code-review

## Tasks

- [x] 1. 定义 WIT 接口 — 创建 `wit/image-processor.wit`，定义 world、resource、enum、record — ref: requirements "成功标准", design "WIT 接口定义"
- [x] 2. 创建 crate 骨架 — `crates/limpuai-wit-image/Cargo.toml` + `src/lib.rs` 基础结构 — ref: requirements "约束", design "依赖配置"
- [x] 3. 实现 decode-image + image-info — 解码入口函数，从字节解析图像，返回 resource handle — ref: requirements "所有默认图像格式解码正确", design "API 合约"
- [x] 4. 实现 image-handle resource — width/height/color-type/pixel-data 元数据获取 + 像素数据导出 — ref: design "image-handle 操作"
- [x] 5. 实现图像操作方法 — resize/crop/blur/brighten/contrast/grayscale/invert/fliph/flipv/rotate90/180/270 — ref: requirements "Resource 接口支持基本操作"
- [x] 6. 实现 encode-image — 将 image-handle 编码为指定格式的字节输出 — ref: requirements "编码输出至少支持 PNG/JPEG/BMP/WebP/TIFF/GIF", design "encode-image"
- [x] 7. 编写测试 — 解码/元数据/操作/编码的 native target 测试 — ref: requirements "Native target 测试通过"
- [x] 8. 集成验证 — 更新 workspace Cargo.toml、cargo build --target wasm32-wasip2、验证 WASM 体积 — ref: requirements "WASM 产物 < 5 MB", design "集成点"

## Notes
- image crate 0.25.x 需 MSRV 1.88.0，确认工具链版本
- avif 格式暂不启用（ravif 体积大）
- 16-bit/32f 像素格式的 pixel-data 返回时字节序为大端（image-rs 内部表示）
- `invert` 操作直接修改原图，与其他操作（返回新 handle）语义不同
- pic-scale-safe 是 image 0.25 的新依赖，需确认其在 wasm32-wasip2 下的编译情况
