# Might-It-Be

Forward-looking thoughts, future TODOs, and design controversies from feature development.

## Future TODO: Arrow 组件体积优化

limpuai-wit-arrow 当前 1.1 MB，包含 arrow-cast 和 arrow-select 等未被 parse 函数使用的模块。可通过直接依赖 arrow-ipc/arrow-array/arrow-schema 子 crate 替代 umbrella crate 来减少体积。但需验证 `ipc` feature 是否传递引入了这些模块。

## Future TODO: 嵌套类型支持

当前 List/Map/Struct 类型统一映射为 `null`。如果上层需要读取 Parquet 嵌套数据（如 JSON 字段），需要扩展 WIT FieldValue variant 或添加专门的嵌套类型。

## Future TODO: WASI runtime 端到端测试

当前测试在 native target 上运行。应增加 wasmtime/wasmtime-provider 端到端测试，验证 wasm 组件在真实 WASI runtime 中的解析正确性。对 polars 组件尤其重要——resource handle 的生命周期管理需要在真实 runtime 中验证。

## Future TODO: Polars 组件体积优化

limpuai-wit-polars 编译到 WASM 后约 26 MB，显著大于 arrow (1.1 MB) 和 parquet (828 KB)。可通过以下方式优化：进一步裁剪 Polars features（如去掉未使用的 dtype-full）、使用 wasm-opt/wasm-strip 后处理、或用 dtype-slim 替代 dtype-full 按需添加具体 dtype。

## Future TODO: Polars to-table 批量导出

当前 `to-table()` 逐行逐列提取 AnyValue，对大 DataFrame 效率低。应改为列式批量提取，或增加分页参数（offset/limit）。也可考虑直接导出 Arrow IPC bytes 跳过 DataTable 中间格式。

## Future TODO: Polars 复合 Filter 条件

当前 filter() 仅支持单列单条件比较（eq/neq/gt/gte/lt/lte）。应在 WIT 接口增加复合条件类型，支持 AND/OR/NOT 组合、范围过滤（between）、字符串模式匹配（contains/starts-with）、null 判断。

## Future TODO: Polars Group-by API 迁移

Polars 0.53 中 GroupBy 的 `.select().sum()` 等方法已标记 deprecated，推荐使用 Lazy API。由于 Lazy API 无法在 WASI 编译，当前只能使用 deprecated 方法。需监控上游发布说明，在 Eager API 有替代方案时及时迁移。

## Future TODO: cargo about 更新 THIRD-PARTY-LICENSES

polars 组件引入了新的依赖树（含 polars 及其子 crate），需运行 `cargo about` 更新 THIRD-PARTY-LICENSES.md 文件。

## Controversy: WIT world 命名

design.md 中定义 world 名称为 `arrow-parser` 和 `parquet-parser`，实际实现为 `arrow-ipc` 和 `parquet-file`。polars 组件使用了 `polars-dataframe`。三者都正确工作，但命名风格不一致。后续新增组件时应统一命名规范。

## Architecture Insight: Polars 组件打破"仅解析"职责边界

arrow/parquet 组件只负责格式解析（bytes → DataTable），不包含数据处理。但 polars 组件导出了完整的 DataFrame 操作能力（select/filter/sort/group-by/join）。这是有意为之的方向 B 决策——Polars 的核心价值在于 DataFrame 操作而非仅仅是解析。这意味着组件职责边界可以按需扩展，不必拘泥于"仅解析"原则。

## Architecture Insight: WASM 单线程下 Lazy API 无优势

Polars Lazy API 的主要优势是查询优化（谓词下推 + 自动并行）。但在 WASM 单线程环境下，并行优化本身无效。因此使用 Eager API 在功能完整性和性能上均无实际损失。如果 WASI 后续版本引入线程支持，应重新评估。

## Architecture Insight: 宿主多实例并行模式

WASI 0.3 无原生多线程，但宿主可通过数据分片 + 多 Component 实例实现并行。具体做法：将大数据集分成 N 份，创建 N 个 polars Component 实例，各自独立处理分片，最后由宿主合并结果。这适合 map-reduce 风格的工作负载。
