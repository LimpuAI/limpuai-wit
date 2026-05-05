# arrow-parquet Tasks

## Progress
Goal: Arrow IPC + Parquet WASI 组件
Status: 5/5 (100%)
Current: 一期全部完成
Next: 无（一期范围内）

## Tasks
- [x] 1. 初始化 Workspace (Cargo.toml + wit/ 目录) + 创建 WIT 定义 - ref: design WIT Structure
- [x] 2. 创建 limpuai-wit-arrow crate - ref: design Crate Structure
- [x] 3. 创建 limpuai-wit-parquet crate - ref: design Crate Structure
- [x] 4. 编译验证 (wasm32-wasip2) + 体积测量 - ref: requirements 成功标准
  - limpuai-wit-arrow.wasm: **1.1 MB**
  - limpuai-wit-parquet.wasm: **828 KB**
- [x] 5. 集成测试（用测试数据验证 parse 输出正确性） - ref: requirements Edge cases
  - arrow: 4 个测试（parse_stream, null_values, empty_stream, invalid_data）
  - parquet: 6 个测试（parse_basic, null_values, empty_file, timestamp_types, multiple_batches, invalid_data）
- ~~6. 回到 deneb-rs 更新 deneb-wit-wasm 引用 limpuai:data 接口~~ — 由 deneb-rs 项目自行处理

## Notes
- parquet crate 使用原生 `parquet::record` API，不依赖 arrow（体积减少 86%）
- parquet dev-dependencies 包含 arrow 仅用于测试数据生成，不影响 wasm 产物
- 两个组件都导入了 `limpuai:data/types` 接口，类型一致性有保证
- WIT world 名称：arrow-ipc, parquet-file（与 design.md 中的 arrow-parser, parquet-parser 不同）
