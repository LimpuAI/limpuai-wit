# polars-component Tasks

## Progress
Goal: 将 Polars 编译为 WASI Component，提供 DataFrame 读取 + 操作能力（含 join）
Status: 9/9 (100%) ✅ ALL COMPLETE
Current: Done
Next: (none)

## Tasks
- [x] 1. 定义 WIT 接口 — `wit/polars-parser.wit` — resource dataframe + parse-csv/parse-json + join + 全部操作方法
- [x] 2. 创建 crate 骨架 — `crates/limpuai-wit-polars/` — Cargo.toml (polars 0.53 + wit-bindgen 0.57)
- [x] 3. 实现 parse-csv + parse-json — CsvReader + JsonReader，SchemaField 类型映射
- [x] 4. 实现 DataFrame 元信息 + 截取操作 — columns/height/width/select/head/tail/unique
- [x] 5. 实现 filter + sort — FilterOp 条件过滤 + 多列排序
- [x] 6. 实现 group-by 聚合 — sum/mean/min/max/count/first/last
- [x] 7. 实现 join 连接 — 接口级函数 join(inner/left/right/full)
- [x] 8. 实现 to-table 导出 — DataFrame → DataTable，Polars DataType → FieldValue
- [x] 9. 编译验证 + 测试 — `cargo build --target wasm32-wasip2 --release` 成功 (26MB wasm) + 24 测试全部通过

## Build Results
- `cargo check -p limpuai-wit-polars` ✅ 零错误
- `cargo build --target wasm32-wasip2 --release -p limpuai-wit-polars` ✅ (stable Rust, 6min16s)
- `target/wasm32-wasip2/release/limpuai_wit_polars.wasm` = 26MB
- `cargo test -p limpuai-wit-polars` ✅ 24 passed, 0 failed

## Notes
- Polars lazy API 不可用（ring/lz4-sys/zstd-sys 不支持 WASI），全部使用 eager API
  - 影响：无查询优化器（谓词下推等），但 WASM 本身单线程，lazy 的并行优化也无效
- 使用 stable Rust 编译（无需 nightly / -Zbuild-std），与现有组件一致
- join 通过接口级函数实现（非 resource 方法），接收两个 dataframe handle
- Polars 在 WASM 目标上自动单线程执行（polars-core/src/lib.rs 已有 cfg fallback）
- WIT resource 模式：Guest trait 关联 type Dataframe = DataframeImpl，通过 DataframeHandle 中间类型包装
