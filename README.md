# limpuai-wit

Arrow IPC 与 Parquet 的 WASI Component 实现，提供标准 WIT 接口供任何 WASI 运行时动态链接使用。

## 为什么构建这个项目

一些优秀的第三方库没有官方的 WIT + WASM 发布，因此本项目承担这一职责：

- 定义 WIT 接口（当前为 `limpuai:data`，后续按需扩展更多 package）
- 将三方库编译为 WASI Component（wasm32-wasip2）

不对三方库源码做任何改动，仅做接口定义和编译导出。

## 特性

- **跨语言复用** — 任何支持 WASI Component Model 的运行时（Rust、Go、Python、JS 等）都可以通过相同的 WIT 接口调用，无需为每种语言绑定原生解析库。
- **沙箱隔离** — 解析逻辑运行在 WASM 沙箱中，宿主无需信任解析器实现的安全性和稳定性。
- **动态链接** — 组件可在运行时按需加载，宿主只需引用对应的 WIT 接口即可对接，不增加编译时依赖。
- **可扩展** — 新的数据格式只需添加对应的解析组件并导出相同的 `parse` 接口，宿主无需任何改动。

## 项目结构

```
wit/                          # 统一 WIT 定义 (limpuai:data)
  types.wit                   # 共享类型: DataTable, FieldValue, SchemaField
  arrow-parser.wit            # Arrow IPC 解析接口 + world
  parquet-parser.wit          # Parquet 解析接口 + world
crates/
  limpuai-wit-arrow/          # Arrow IPC 解析组件
  limpuai-wit-parquet/        # Parquet 解析组件
```

## WIT 接口

### 共享类型 (`types`)

```wit
record schema-field { name: string, data-type: string }
record data-table { columns: list<schema-field>, rows: list<list<field-value>> }
variant field-value { numeric(f64), text(string), timestamp(f64), boolean(bool), null }
```

### 解析接口

两个组件导出相同的函数签名：

```wit
parse: func(data: list<u8>) -> result<data-table, string>
```

输入原始文件字节，输出结构化 `DataTable`。

## 类型映射

| Arrow 类型 | WIT FieldValue |
|---|---|
| Int8/16/32/64, UInt8/16/32/64, Float32/64 | `numeric(f64)` |
| Boolean | `boolean(bool)` |
| Utf8, LargeUtf8 | `text(string)` |
| Date32, Date64, Timestamp(*, _) | `timestamp(f64)` (毫秒) |
| Null | `null` |

## 构建

需要 `wasm32-wasip2` target：

```bash
rustup target add wasm32-wasip2
cargo build --target wasm32-wasip2 --release
```

产出：
- `target/wasm32-wasip2/release/limpuai_wit_arrow.wasm` (~1.1 MB)
- `target/wasm32-wasip2/release/limpuai_wit_parquet.wasm` (~6.1 MB)

## 测试

```bash
cargo test
```

## 依赖

- `arrow` 54.x (Apache-2.0)
- `parquet` 54.x (Apache-2.0)
- `wit-bindgen` 0.57

## License

Apache-2.0
