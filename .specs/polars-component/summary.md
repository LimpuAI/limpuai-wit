# polars-component Feature Summary

**Date**: 2026-05-06
**Branch**: feature/polars-component
**Status**: ✅ Complete (8/9 success criteria met, 1 deferred)

## Goal

将 Polars Rust 库（crates.io）编译为 WASI Component (wasm32-wasip2)，通过 WIT resource 接口导出完整 DataFrame 操作能力：CSV/JSON 解析 + select/filter/sort/group-by/join + DataTable 导出。

## Deliverables

| 产出 | 路径 | 说明 |
|------|------|------|
| WIT 接口 | `wit/polars-parser.wit` | resource dataframe + 11 方法 + 3 工厂/接口函数 |
| Crate | `crates/limpuai-wit-polars/` | Cargo.toml + src/lib.rs (846 行，含 24 测试) |
| WASM 产物 | `target/wasm32-wasip2/release/limpuai_wit_polars.wasm` | ~26 MB |
| README | `crates/limpuai-wit-polars/README.md` | 接口说明 + 8 项局限性 + 8 项待改进 |
| Spec 文档 | `.specs/polars-component/` | requirements/design/tasks |

## Metrics

| 指标 | 值 |
|------|-----|
| 实现代码行数 | 399 行（不含测试） |
| 测试用例 | 24 (全部通过) |
| WASM 体积 | ~26 MB |
| 编译时间 | ~6 min (release) |
| Polars 版本 | 0.53 |
| wit-bindgen 版本 | 0.57 |
| Rust 工具链 | stable (无需 nightly) |
| WIT 方法数 | 11 (resource 方法) + 3 (接口级函数) |
| 聚合函数 | 7 (sum/mean/min/max/count/first/last) |
| Join 类型 | 4 (inner/left/right/full) |
| 数据类型映射 | Int/UInt/Float/Bool/String/Date/Datetime/Duration/Time |

## Success Criteria Checklist

| # | 标准 | 状态 | 备注 |
|---|------|------|------|
| 1 | WIT 接口定义 | ✅ | `polars-parser.wit`，resource dataframe + parse-csv/parse-json + join |
| 2 | crates.io polars 依赖 | ✅ | `polars = "0.53"`，非本地 path |
| 3 | 编译到 wasm32-wasip2 | ✅ | stable Rust，无 nightly |
| 4 | CSV 解析正确 | ✅ | 测试 `parse_csv_basic` |
| 5 | JSON/NDJSON 解析正确 | ✅ | 测试 `parse_json_basic` + `parse_ndjson` |
| 6 | DataFrame 操作暴露 | ✅ | select/filter/sort/head/tail/unique/group-by/to-table |
| 7 | 操作链可用 | ✅ | 测试 `chain_filter_sort_head` |
| 8 | 测试覆盖 | ✅ | 24 测试全通过 |
| 9 | cargo about 更新 | ⏳ | 延迟到统一执行 |

## Key Technical Decisions

1. **Eager API only** — lazy 依赖 ring/lz4-sys/zstd-sys，无法编译到 WASI。WASM 单线程下 lazy 的并行优化也无效
2. **WIT resource 模式** — `Guest` trait 关联 `type Dataframe = DataframeImpl`，通过 `DataframeHandle` 中间类型包装
3. **join 为接口级函数** — WIT resource 方法只能操作 self，join 需要两个 dataframe handle
4. **Stable Rust 编译** — 实际验证发现无需 nightly/-Zbuild-std，与现有组件一致
5. **RefCell 内部可变性** — WIT resource 方法接收 `&self`，用 `RefCell<DataFrame>` 包装
6. **单文件实现** — 全部逻辑在 `lib.rs`，未拆分为子模块（代码量适中，拆分收益不大）

## Limitations (from README)

1. WASM 体积大 (~26 MB)
2. 不支持 Lazy API (查询优化)
3. 无多线程并行
4. 逐行 to-table 性能
5. Filter 仅支持单条件
6. Group-by 使用 deprecated API
7. Join 不支持复杂场景
8. RefCell 内存管理（避免嵌套借用）

## Files Changed

```
wit/polars-parser.wit                          # 新增 WIT 接口
crates/limpuai-wit-polars/Cargo.toml           # 新增 crate 配置
crates/limpuai-wit-polars/src/lib.rs           # 新增实现 + 测试
crates/limpuai-wit-polars/README.md            # 新增文档
Cargo.toml                                     # workspace members 添加 polars
Cargo.lock                                     # 依赖锁定更新
```

## Deferred Items

- `cargo about` 更新 THIRD-PARTY-LICENSES（需包含 polars 及其子 crate 的许可证）
