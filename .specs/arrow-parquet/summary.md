# Arrow-Parquet WASI Component — Feature Summary

## Goal
将 Arrow IPC 和 Parquet Rust 库编译为 WASI Component，提供标准 WIT 接口 (`limpuai:data`) 供任何项目复用。

## Deliverables

| 产物 | 说明 |
|------|------|
| `wit/types.wit` | 共享类型：DataTable, FieldValue, SchemaField |
| `wit/arrow-parser.wit` | Arrow IPC 解析接口 (world: arrow-ipc) |
| `wit/parquet-parser.wit` | Parquet 解析接口 (world: parquet-file) |
| `crates/limpuai-wit-arrow/` | Arrow IPC 解析组件 → **1.1 MB** wasm |
| `crates/limpuai-wit-parquet/` | Parquet 解析组件 → **828 KB** wasm |

## Metrics

| 指标 | 值 |
|------|-----|
| 测试数量 | 10 (arrow: 4, parquet: 6) |
| wasm 总体积 | ~1.9 MB |
| 依赖许可证 | 129 crates, 6 种许可证 |
| WIT 接口 | `limpuai:data` package |

## Key Design Decisions

1. **Parquet 原生 API**: 使用 `parquet::record` 替代 `parquet::arrow`，体积从 ~6.1 MB 降至 828 KB
2. **统一 WIT package**: 所有接口在 `limpuai:data` 下，规避 wit-bindgen 单 path 限制
3. **各自 world**: 每个组件独立编译为 .wasm，按需加载

## Test Coverage

- 基本类型解析 (Int64, Utf8, Float64, Boolean)
- Null 值处理
- 空文件处理
- 时间戳类型转换 (ms/us)
- 多 batch 合并读取
- 无效数据拒绝

## Success Criteria — All Met

- [x] 共享 WIT 类型定义
- [x] arrow-parser 编译到 wasm32-wasip2
- [x] parquet-parser 编译到 wasm32-wasip2
- [x] 宿主动态链接
- [x] WIT 接口复用
- [x] THIRD-PARTY-LICENSES 生成

## Out of Scope (一期)

- deneb-rs 引用更新 — 由 deneb-rs 项目自行处理
- Arrow 组件体积优化 (arrow-cast/select 未排除)
- List/Map/Struct 嵌套类型 → `null`
- WASI runtime 端到端集成测试
