# limpuai-wit-image Feature Summary

## 目标
将 `image-rs/image` (0.25.x) 编译为 WASI Component (wasm32-wasip2)，提供图像解码、基本操作和编码能力。

## 完成状态
✅ 全部完成 — 2026-05-07

## 交付物

| 文件 | 说明 |
|------|------|
| `wit/image-processor.wit` | WIT 接口定义（interface + world + resource + enum + record） |
| `crates/limpuai-wit-image/Cargo.toml` | 依赖配置（image 0.25, 无 rayon, 13 种格式 feature） |
| `crates/limpuai-wit-image/src/lib.rs` | 核心实现（306 行 + 21 测试） |
| `Cargo.toml` | workspace members 更新 |
| `README.md` | 文档更新（架构图、组件表、WIT 接口、类型映射） |

## 关键指标

| 指标 | 值 |
|------|-----|
| WASM 产物大小 | 3.2 MB |
| 测试数量 | 21 个（全部通过） |
| 支持解码格式 | PNG, JPEG, GIF, WebP, TIFF, BMP, ICO, PNM, QOI, TGA, HDR, FF, EXR |
| 支持编码格式 | PNG, JPEG, BMP, GIF, TIFF, ICO, WebP, PNM |
| 图像操作 | 12 种（resize, crop, blur, brighten, contrast, grayscale, invert, fliph, flipv, rotate90/180/270） |
| rayon 依赖 | 无 |
| std::fs 依赖 | 无（仅使用 load_from_memory + Cursor） |

## WIT 接口

- **Interface**: `image-processor`（package `limpuai:data`）
- **World**: `image-data`
- **Resource**: `image-handle`（16 个方法）
- **Top-level functions**: `decode-image`, `encode-image`, `image-info`
- **Enums**: `color-type`（10 种）, `output-format`（8 种）
- **Records**: `image-meta`

## Wave 执行记录

| Wave | Tasks | Agents | 耗时 |
|------|-------|--------|------|
| W1 | Task 1 (WIT) + Task 2 (Crate 骨架) | 2 并行 | ~3 min |
| W2 | Task 3+4+5+6 (核心实现) | 1 deep | ~10 min |
| W3 | Task 7+8 (测试+集成) | 1 deep | ~7 min |
| Review | spec-code-review | 1 oracle | ~5 min |
| Fix | P1 修正（5 项） | 直接执行 | ~2 min |

## 设计决策

1. **Resource 模式**: 解码一次，多次操作/编码，避免重复解码
2. **不可变操作**: 所有操作返回新 handle，原图不受影响（invert 除外）
3. **encode-image by-value**: handle 被 consume，宿主需 re-decode 以多次编码
4. **禁用 rayon**: WASM 单线程，`default-features = false`
5. **禁用 avif**: ravif 体积大，暂不启用

## 已知限制

- `encode-image` consume handle — 无法对同一图像连续编码不同格式
- `invert` 原地修改 + 返回克隆 — WIT 签名与设计意图的妥协
- `brighten` f32→i32 截断（已用 `.round()` 缓解）
- 无 AVIF 格式支持
