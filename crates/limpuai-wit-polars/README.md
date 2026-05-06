# limpuai-wit-polars

将 [Polars](https://pola.rs/) DataFrame 库编译为 WASI Component，通过 WIT resource 接口导出完整的 DataFrame 操作能力。

## 概述

本组件在 WASM 沙箱中提供 Polars 的核心 DataFrame 功能——CSV/JSON 解析、列选择、过滤、排序、聚合、连接、去重和导出。宿主通过 WIT 接口操作 resource handle，无需关心 Polars 内部实现。

## WIT 接口

接口定义在 `wit/polars-parser.wit`，核心结构：

```wit
resource dataframe {
    columns: func() -> list<schema-field>;
    height: func() -> u64;
    width: func() -> u64;
    select: func(columns: list<string>) -> result<dataframe, string>;
    filter: func(column: string, op: filter-op, value: field-value) -> result<dataframe, string>;
    sort: func(by: list<sort-option>) -> result<dataframe, string>;
    head: func(n: u64) -> result<dataframe, string>;
    tail: func(n: u64) -> result<dataframe, string>;
    unique: func(subset: option<list<string>>) -> result<dataframe, string>;
    group-by: func(by: list<string>, aggregations: list<aggregation>) -> result<dataframe, string>;
    to-table: func() -> result<data-table, string>;
}

parse-csv: func(data: list<u8>) -> result<dataframe, string>;
parse-json: func(data: list<u8>) -> result<dataframe, string>;
join: func(left: dataframe, right: dataframe, left-on: list<string>, right-on: list<string>, how: join-type) -> result<dataframe, string>;
```

### 共享类型（来自 `types.wit`）

| 类型 | 用途 |
|------|------|
| `data-table` | 列式存储的表格数据，作为最终导出格式 |
| `field-value` | 变体类型：`numeric(f64)`, `text(string)`, `timestamp(f64)`, `boolean(bool)`, `null` |
| `schema-field` | 列描述：`{ name, data-type }` |

### 类型映射

| Polars DataType | WIT FieldValue |
|---|---|
| Int8/16/32/64, UInt8/16/32/64 | `numeric(f64)` |
| Float32, Float64 | `numeric(f64)` |
| Boolean | `boolean(bool)` |
| String | `text(string)` |
| Date | `timestamp(f64)` (天 × 86400000 → 毫秒) |
| Datetime(Milliseconds, _) | `timestamp(f64)` (毫秒) |
| Datetime(Microseconds, _) | `timestamp(f64)` (÷1000 → 毫秒) |
| Datetime(Nanoseconds, _) | `timestamp(f64)` (÷1000000 → 毫秒) |
| Duration(TimeUnit) | `numeric(f64)` (转换为毫秒) |
| Time | `numeric(f64)` (原始纳秒值) |
| Null / 其他 | `null` |

## 构建

```bash
cargo build --target wasm32-wasip2 --release -p limpuai-wit-polars
```

使用 stable Rust，无需 nightly 工具链。产出：

- `target/wasm32-wasip2/release/limpuai_wit_polars.wasm` (~26 MB)

## 测试

```bash
cargo test -p limpuai-wit-polars
```

测试覆盖：CSV/JSON 解析、select、filter（数值/字符串/布尔）、sort（升序/降序）、head/tail、unique、group-by（含多聚合和别名）、join（inner/left/full）、to-table 类型映射、链式操作。

## 依赖

- `polars` 0.53 (MIT license)
- `wit-bindgen` 0.57 (Apache-2.0)

### 启用的 Polars Features

`csv`, `json`, `temporal`, `dtype-slim`, `dtype-full`, `rows`, `zip_with`, `round_series`, `is_in`, `diff`, `abs`, `cum_agg`, `rolling_window`, `unique_counts`, `diagonal_concat`, `strings`

## 当前局限性

### 1. WASM 体积较大（~26 MB）

Polars 是一个功能丰富的库，即使用 `default-features = false` 精选 feature，编译到 WASM 后仍然显著大于 Arrow（~1.1 MB）和 Parquet（~6.1 MB）组件。主要原因：

- Polars 内部依赖大量泛型展开和向量化操作
- `dtype-full` feature 引入了所有数据类型的支持代码
- `temporal` feature 包含完整的时序处理逻辑

**影响**：加载和实例化时间较长，不适合对启动延迟敏感的场景。

**可能的改进**：
- 进一步裁剪 feature 集合，只保留实际使用的 dtype
- 考虑 `dtype-slim` 替代 `dtype-full`，按需添加具体 dtype feature
- 探索 wasm-opt / wasm-strip 进行二进制优化

### 2. 不支持 Lazy API（查询优化）

Polars 的 Lazy API 依赖 `ring`（TLS）、`lz4-sys`、`zstd-sys`（C 编译）等 crate，它们无法编译到 WASI 目标。因此本组件仅使用 Eager API。

**影响**：
- 无查询优化器（谓词下推、投影下推、切片下推等）
- 无法使用 `scan_csv`/`scan_parquet` 等延迟扫描方法
- 每次 `select`/`filter`/`sort` 操作都会立即执行全量计算

**但在 WASM 环境下影响有限**：
- Polars 在 `cfg(not(target_family = "wasm"))` 时才启用多线程并行
- WASM 单线程环境下，Lazy API 的并行优化本身也无效
- 对于中小数据集，Eager API 的性能差距不大

### 3. 无多线程并行

WASI Preview 2 (wasm32-wasip2) 没有原生多线程支持。Polars 在 WASM 目标上自动回退到单线程执行（`polars-core/src/lib.rs` 中有 `cfg` fallback）。

**影响**：
- 大数据集的排序、聚合、join 操作无法利用多核
- 单个 Component 实例的计算能力受限于单线程

**宿主侧可能的改进方案**：
- 数据分片 + 多个 Component 实例并行执行
- WASI 0.3 后续版本可能引入线程支持，届时可重新评估

### 4. 逐行导出性能（to-table）

`to-table()` 通过逐行逐列提取 `AnyValue` 转换为 `DataTable`，对大 DataFrame 效率较低。

**影响**：数万行以上的 DataFrame 调用 `to-table()` 会有明显延迟。

**可能的改进**：
- 批量提取整列数据，避免逐行 `Series::get()`
- 支持分页/流式导出（修改 WIT 接口增加 offset/limit 参数）
- 直接导出列式结构（Arrow IPC bytes），由宿主解析

### 5. Filter 操作受限

当前 `filter()` 仅支持单列、单条件的比较过滤（eq/neq/gt/gte/lt/lte），不支持：

- 多条件组合（AND/OR）
- 范围过滤（between）
- 字符串模式匹配（contains/starts-with/regex）
- Null 值判断（is-null/is-not-null）

**可能的改进**：在 WIT 接口中增加复合条件类型（如 `filter-condition` variant），支持逻辑组合。

### 6. Group-by 聚合使用 deprecated API

`GroupBy::select().sum()` 等方法在 Polars 0.53 中已标记为 deprecated，推荐使用 Lazy API 的聚合方式。由于 Lazy API 不可用（见第 2 点），当前只能使用 deprecated 方法。

**影响**：未来 Polars 版本可能移除这些方法，需要跟进上游变更。

**可能的改进**：监控 Polars 发布说明，在 Eager API 有替代方案时及时迁移。

### 7. Join 不支持复杂场景

当前 `join()` 仅支持等值连接（equi-join），不支持：

- 非等值连接（如 range join）
- 交叉连接（cross join）
- Join 后的去重策略（coalesce）
- 自定义后缀处理重名列

### 8. 内存管理

DataFrame 使用 `RefCell` 包装以实现内部可变性（WIT resource 方法接收 `&self`）。在 WASM 单线程环境下不存在并发风险，但如果同一 handle 被多次借用（如嵌套调用），可能触发 `RefCell` panic。

**影响**：正常使用不会触发，但需要避免在同一 DataFrame 上进行递归或嵌套操作。

## 待改进项

| 优先级 | 项目 | 说明 |
|--------|------|------|
| 高 | WASM 体积优化 | 裁剪 features、wasm-opt、tree-shaking |
| 高 | 批量 to-table | 列式批量提取替代逐行 `Series::get()` |
| 中 | 复合 filter 条件 | WIT 接口增加 AND/OR/NOT 组合条件 |
| 中 | 分页导出 | to-table 增加 offset/limit 参数 |
| 中 | 更多聚合函数 | median, std, var, n_unique 等 |
| 低 | NDJSON 解析参数化 | 暴露 `with_json_format` 到 WIT 接口 |
| 低 | CSV 解析参数化 | 暴露分隔符、有无 header、schema 提示等 |
| 低 | DataFrame 持久化 | serialize/deserialize 为字节，支持跨调用缓存 |

## License

Apache-2.0
