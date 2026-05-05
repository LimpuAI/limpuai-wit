# arrow-parquet Requirements

## What we need
将 crates.io 上的 Arrow IPC 和 Parquet Rust 库编译为 WASI Component，提供标准 WIT 接口供任何项目复用。

## 一期范围
- Arrow IPC 解析（依赖 `arrow` crate）
- Parquet 解析（依赖 `parquet` crate）

## Input & Output
**Input**: Arrow IPC 或 Parquet 文件原始字节 (`list<u8>`)
**Output**: 解析后的 DataTable（共享类型）

## Success criteria
- [x] 共享 WIT 类型定义（DataTable、FieldValue、SchemaField）
- [x] arrow-parser 组件编译到 wasm32-wasip2，导出 parse 函数
- [x] parquet-parser 组件编译到 wasm32-wasip2，导出 parse 函数
- [x] 宿主可通过运行时动态链接使用这两个组件
- [x] 任何项目只需引用相同的 WIT 接口即可对接
- [ ] 使用 `cargo about` 生成 `THIRD-PARTY-LICENSES` 文件（含上游 crate 许可声明）

## Edge cases
- [x] 无效数据格式返回明确错误信息
- [x] 空 Arrow/Parquet 文件不崩溃
