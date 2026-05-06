# Get-It-Done

Resolved items from might-it-be.md with resolution context.

## Resolved: Parquet 组件体积优化 (2026-05-05)

**Original**: parquet 组件使用 `parquet::arrow` API，体积 ~6.1 MB。

**Resolution**: 改用 parquet 原生 `parquet::record` API，去掉 arrow 依赖。体积降至 828 KB，减少 86%。类型覆盖保持完整（Bool, Int, Long, Float, Double, Str, Bytes, Date, TimestampMillis, TimestampMicros）。测试通过 dev-dependencies 中的 arrow writer 生成数据，不影响 wasm 产物。

## Resolved: 许可证文件生成 (2026-05-06)

**Original**: requirements 要求 `cargo about` 生成 THIRD-PARTY-LICENSES。

**Resolution**: 安装 cargo-about，配置 about.toml 接受 Apache-2.0/MIT/BSD-3-Clause/Unicode-3.0/Zlib/CC0-1.0，生成 THIRD-PARTY-LICENSES.md (2689 行)。同时为 workspace 和子 crate 添加了 `license = "Apache-2.0"` 字段。

## Resolved: Polars 组件职责边界 (2026-05-06)

**Original**: might-it-be.md 中记录"WASI 组件应只负责格式解析，不包含数据处理能力"。

**Resolution**: polars 组件有意选择方向 B（完整 DataFrame 操作导出），打破"仅解析"限制。Polars 的核心价值在于 DataFrame 操作（筛选、聚合、连接），仅做解析无法体现其能力。WIT resource 模式天然支持有状态操作链，技术实现证明此方案可行。原 architecture insight 更新为"组件职责边界可以按需扩展，不必拘泥于仅解析原则"。

## Resolved: Polars 构建方式 (2026-05-06)

**Original**: design.md 规划使用 nightly + `-Zbuild-std=core,alloc,std,panic_abort` 构建。

**Resolution**: 实际验证发现 stable Rust 即可编译到 wasm32-wasip2，无需 nightly 工具链。这与 arrow/parquet 组件的构建方式一致，降低了工具链要求。design.md 中 Build 章节已过时，实际只需 `cargo build --target wasm32-wasip2 --release`。
