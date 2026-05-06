# arrow-parquet Design

## WIT Structure
统一 package `limpuai:data`，所有接口和类型在同一 package 下。

```
wit/
  types.wit              # 共享数据类型
  arrow-parser.wit       # Arrow IPC 解析接口 + world
  parquet-parser.wit     # Parquet 解析接口 + world
```

### types.wit
```wit
package limpuai:data;
interface types {
    record schema-field { name: string, data-type: string }
    record data-table { columns: list<schema-field>, rows: list<list<field-value>> }
    variant field-value { numeric(f64), text(string), timestamp(f64), boolean(bool), null }
}
```

### arrow-parser.wit
```wit
package limpuai:data;
interface arrow-parser {
    use types.{data-table};
    parse: func(data: list<u8>) -> result<data-table, string>;
}
world arrow-parser { export arrow-parser; }
```

### parquet-parser.wit
```wit
package limpuai:data;
interface parquet-parser {
    use types.{data-table};
    parse: func(data: list<u8>) -> result<data-table, string>;
}
world parquet-parser { export parquet-parser; }
```

## Crate Structure
```
limpuai-wit/
  wit/                        # 统一 WIT 定义
    types.wit
    arrow-parser.wit
    parquet-parser.wit
  crates/
    limpuai-wit-arrow/        # Arrow IPC 解析组件
      Cargo.toml              # 依赖 arrow crate + wit-bindgen
      src/lib.rs
    limpuai-wit-parquet/      # Parquet 解析组件
      Cargo.toml              # 依赖 parquet crate + wit-bindgen
      src/lib.rs
```

## Key decisions
- **统一 package**: 所有 WIT 文件在 `limpuai:data` 下，规避 wit-bindgen 0.51 单 path 限制
- **各自 world**: 每个组件有自己的 world（arrow-parser、parquet-parser），编译为独立 .wasm
- **共享类型**: DataTable/FieldValue/SchemaField 在 types 接口中定义一次，所有组件 `use` 引用
- **命名空间**: `limpuai:data` — 任何项目的公共数据类型和解析器

## Type Mapping (Arrow → WIT)
- Int8/16/32/64, UInt8/16/32/64 → `numeric(f64)`
- Float32/64 → `numeric(f64)`
- Boolean → `boolean(bool)`
- Utf8/LargeUtf8 → `text(string)`
- Date32/Date64, Timestamp → `timestamp(f64)` (毫秒)
- Null → `null`

## Type Mapping (Parquet → WIT)
- Bool → `boolean(bool)`
- Byte/Short/Int/Long, UByte/UShort/UInt/ULong → `numeric(f64)`
- Float16/Float/Double → `numeric(f64)`
- Str → `text(string)`
- Bytes → `text(string)` (hex 编码)
- Date → `timestamp(f64)` (天数 × 86400000 → 毫秒)
- TimestampMillis → `timestamp(f64)` (毫秒)
- TimestampMicros → `timestamp(f64)` (微秒 ÷ 1000 → 毫秒)
- Decimal → `text(string)` ("[decimal]")
- Group/ListInternal/MapInternal → `null`

## Dependencies
- `arrow` 54.x (Apache-2.0) — Arrow IPC 解析
- `parquet` 54.x (Apache-2.0) — Parquet 文件解析（原生 `parquet::record` API，不使用 arrow feature）
- `wit-bindgen` 0.57 — WIT 绑定生成
