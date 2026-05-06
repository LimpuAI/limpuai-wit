# Might-It-Be

Forward-looking thoughts, future TODOs, and design controversies from arrow-parquet implementation.

## Future TODO: Arrow 组件体积优化

limpuai-wit-arrow 当前 1.1 MB，包含 arrow-cast 和 arrow-select 等未被 parse 函数使用的模块。可通过直接依赖 arrow-ipc/arrow-array/arrow-schema 子 crate 替代 umbrella crate 来减少体积。但需验证 `ipc` feature 是否传递引入了这些模块。

## Future TODO: 嵌套类型支持

当前 List/Map/Struct 类型统一映射为 `null`。如果上层需要读取 Parquet 嵌套数据（如 JSON 字段），需要扩展 WIT FieldValue variant 或添加专门的嵌套类型。

## Future TODO: WASI runtime 端到端测试

当前测试在 native target 上运行。应增加 wasmtime/wasmtime-provider 端到端测试，验证 wasm 组件在真实 WASI runtime 中的解析正确性。

## Controversy: WIT world 命名

design.md 中定义 world 名称为 `arrow-parser` 和 `parquet-parser`，实际实现为 `arrow-ipc` 和 `parquet-file`。两者都正确工作，但命名风格不一致。后续新增组件时应统一命名规范。

## Architecture Insight: 组件职责边界

WASI 组件应只负责格式解析，不包含数据处理能力（类型转换、过滤等）。这保持了小体积和清晰的接口职责。上层宿主应自行处理数据转换逻辑。
