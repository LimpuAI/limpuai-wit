# Get-It-Done

Resolved items from might-it-be.md with resolution context.

## Resolved: Parquet 组件体积优化 (2026-05-05)

**Original**: parquet 组件使用 `parquet::arrow` API，体积 ~6.1 MB。

**Resolution**: 改用 parquet 原生 `parquet::record` API，去掉 arrow 依赖。体积降至 828 KB，减少 86%。类型覆盖保持完整（Bool, Int, Long, Float, Double, Str, Bytes, Date, TimestampMillis, TimestampMicros）。测试通过 dev-dependencies 中的 arrow writer 生成数据，不影响 wasm 产物。

## Resolved: 许可证文件生成 (2026-05-06)

**Original**: requirements 要求 `cargo about` 生成 THIRD-PARTY-LICENSES。

**Resolution**: 安装 cargo-about，配置 about.toml 接受 Apache-2.0/MIT/BSD-3-Clause/Unicode-3.0/Zlib/CC0-1.0，生成 THIRD-PARTY-LICENSES.md (2689 行)。同时为 workspace 和子 crate 添加了 `license = "Apache-2.0"` 字段。
