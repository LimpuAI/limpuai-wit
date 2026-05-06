# polars-component Design

## WIT Structure

在现有 `limpuai:data` package 下新增 polars 接口，复用共享类型。

```
wit/
  types.wit                 # 现有共享类型 (不变)
  arrow-parser.wit          # 现有 Arrow IPC (不变)
  parquet-parser.wit        # 现有 Parquet (不变)
  polars-parser.wit         # 新增: Polars DataFrame 接口 + world
```

### polars-parser.wit

```wit
package limpuai:data;

interface polars-parser {
    use types.{data-table, field-value, schema-field};

    /// 筛选操作符
    enum filter-op {
        eq,          // ==
        neq,         // !=
        gt,          // >
        gte,         // >=
        lt,          // <
        lte,         // <=
    }

    /// 聚合函数
    enum agg-fn {
        sum,
        mean,
        min,
        max,
        count,
        first,
        last,
    }

    /// 排序方向
    record sort-option {
        column: string,
        descending: bool,
        null-last: bool,
    }

    /// 聚合请求
    record aggregation {
        column: string,
        function: agg-fn,
        alias: option<string>,
    }

    /// Join 类型
    enum join-type {
        inner,
        left,
        right,
        full,
    }

    /// DataFrame 操作接口 (resource)
    resource dataframe {
        /// 获取列元信息
        columns: func() -> list<schema-field>;
        /// 行数
        height: func() -> u64;
        /// 列数
        width: func() -> u64;

        /// 按列名选取子集
        select: func(columns: list<string>) -> result<dataframe, string>;
        /// 按条件过滤行
        filter: func(column: string, op: filter-op, value: field-value) -> result<dataframe, string>;
        /// 排序
        sort: func(by: list<sort-option>) -> result<dataframe, string>;
        /// 取前 N 行
        head: func(n: u64) -> result<dataframe, string>;
        /// 取后 N 行
        tail: func(n: u64) -> result<dataframe, string>;
        /// 去重
        unique: func(subset: option<list<string>>) -> result<dataframe, string>;

        /// 分组聚合
        group-by: func(by: list<string>, aggregations: list<aggregation>) -> result<dataframe, string>;

        /// 导出为 DataTable
        to-table: func() -> result<data-table, string>;
    }

    /// 从 CSV 字节创建 DataFrame
    parse-csv: func(data: list<u8>) -> result<dataframe, string>;
    /// 从 JSON/NDJSON 字节创建 DataFrame
    parse-json: func(data: list<u8>) -> result<dataframe, string>;

    /// 连接两个 DataFrame（接口级函数，接收两个 resource handle）
    join: func(left: dataframe, right: dataframe,
               left-on: list<string>, right-on: list<string>,
               how: join-type) -> result<dataframe, string>;
}

world polars-dataframe {
    export polars-parser;
}
```

## Crate Structure

```
limpuai-wit/
  wit/
    types.wit
    arrow-parser.wit
    parquet-parser.wit
    polars-parser.wit          # 新增
  crates/
    limpuai-wit-arrow/         # 现有
    limpuai-wit-parquet/       # 现有
    limpuai-wit-polars/        # 新增
      Cargo.toml
      src/
        lib.rs                  # Component 入口 + parse 函数
        dataframe.rs            # DataFrame resource 实现
        convert.rs              # Polars DataType → WIT FieldValue 转换
```

## Key decisions

| 决策 | 选择 | 理由 | 备选 |
|------|------|------|------|
| WIT 接口风格 | Resource-based | DataFrame 是有状态对象，操作链需要保持中间状态 | 函数式（每次传 bytes，不实际） |
| Polars 依赖来源 | crates.io `polars` crate | 项目原则：不修改源码，标准依赖 | 本地 path（违反原则） |
| Polars features | `csv,json,temporal,dtype-slim,rows` + 额外 eager ops | 已验证编译通过，覆盖核心操作 | lazy（ring 不支持 WASI） |
| DataFrame 操作 | Eager API only | lazy 依赖 ring/lz4-sys/zstd-sys，无法编译到 WASI | lazy API（不可行） |
| 构建方式 | `cargo build --target wasm32-wasip2 --release` | stable Rust 即可编译，与 arrow/parquet 一致 | nightly + `-Zbuild-std`（实际不需要） |
| Join 设计 | 接口级函数 `join(left, right, ...)` | WIT 允许接口函数接收 resource 参数，实现跨 DataFrame 操作 | resource 方法（只能操作 self） |

## API contracts

### parse-csv
**输入**: CSV 文件原始字节 (`list<u8>`)
**输出**: `dataframe` resource handle 或错误字符串
**错误**: 无效 CSV 格式、IO 错误

### parse-json
**输入**: JSON/NDJSON 文件原始字节 (`list<u8>`)
**输出**: `dataframe` resource handle 或错误字符串
**错误**: 无效 JSON 格式、schema 推断失败

### dataframe.select
**输入**: 列名列表
**输出**: 新 `dataframe` resource handle（仅包含指定列）
**错误**: 列名不存在

### dataframe.filter
**输入**: 列名 + 比较操作符 + 比较值
**输出**: 新 `dataframe` resource handle（过滤后的行）
**错误**: 列名不存在、类型不匹配

### dataframe.group-by
**输入**: 分组列名列表 + 聚合操作列表
**输出**: 新 `dataframe` resource handle（聚合结果）
**错误**: 列名不存在、聚合函数不适用的类型

### dataframe.to-table
**输入**: 无
**输出**: `DataTable`（可用于其他组件或宿主消费）
**错误**: 转换失败

### join (接口级函数)
**输入**: 两个 `dataframe` handle + 左表连接键 + 右表连接键 + join 类型
**输出**: 新 `dataframe` resource handle（连接结果）
**错误**: key 列不存在、类型不匹配、join 类型不支持

## Type Mapping (Polars → WIT)

| Polars DataType | WIT FieldValue |
|---|---|
| Int8, Int16, Int32, Int64 | `numeric(f64)` |
| UInt8, UInt16, UInt32, UInt64 | `numeric(f64)` |
| Float32, Float64 | `numeric(f64)` |
| Boolean | `boolean(bool)` |
| String, Utf8 | `text(string)` |
| Date | `timestamp(f64)` (天数 × 86400000 → 毫秒) |
| Datetime(_, _) | `timestamp(f64)` (毫秒) |
| Duration(_) | `numeric(f64)` (毫秒) |
| Time | `numeric(f64)` (纳秒) |
| Null | `null` |
| List, Struct, Array, Categorical, Decimal, Binary | `null` |

## Dependencies

```toml
[dependencies]
polars = { version = "0.53", default-features = false, features = [
    "csv",
    "json",
    "temporal",
    "dtype-slim",
    "rows",
    "zip_with",
    "round_series",
    "is_in",
    "diff",
    "abs",
    "cum_agg",
    "rolling_window",
    "unique_counts",
    "diagonal_concat",
    "strings",
    "dtype-full",
] }
wit-bindgen = "0.57"
```

## Build

使用 stable Rust 编译（与 arrow/parquet 组件一致），无需 nightly：

```bash
cargo build --target wasm32-wasip2 --release -p limpuai-wit-polars
```

> **注**: 初始设计规划使用 nightly + `-Zbuild-std`，但实际验证发现 stable Rust 即可编译到 wasm32-wasip2。

## Integration points

- 复用 `wit/types.wit` 中的 `DataTable`/`FieldValue`/`SchemaField` 类型
- 与现有 arrow/parquet 组件共享 `limpuai:data` package 命名空间
- 宿主通过 WIT resource 持有 `dataframe` handle，链式调用操作方法
