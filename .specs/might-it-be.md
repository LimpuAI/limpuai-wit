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

## Future TODO: Symphonia 升级到 0.6

Symphonia 0.5 不提供 `FormatReader::format_info()` 方法（0.6 新增），导致无法从 FormatReader 获取实际容器格式名。当前通过 magic bytes 检测（`detect_format_name()`）作为 workaround。升级到 0.6 后可直接使用 `format_info().short_name`，且支持更多格式。但 0.6 目前为 alpha，需等 stable 发布。

## Future TODO: Symphonia seek 超出范围测试

cc-review 发现 seek 超出文件 duration 时缺少测试。实现会返回 Symphonia 的原始错误消息，而非 requirements 中建议的 "seek beyond end"。应增加边界测试并考虑统一错误消息格式。

## Future TODO: Symphonia 大文件端到端测试

当前测试使用 ~100 帧 fixture 文件。应增加大文件（>1MB）的 decode-chunk 流式测试，验证分块模式在多 packet 场景下 leftover buffer 正确工作。

## Controversy: WIT world 命名

design.md 中定义 world 名称为 `arrow-parser` 和 `parquet-parser`，实际实现为 `arrow-ipc` 和 `parquet-file`。polars 组件使用了 `polars-dataframe`。三者都正确工作，但命名风格不一致。后续新增组件时应统一命名规范。

## Architecture Insight: Polars 组件打破"仅解析"职责边界

arrow/parquet 组件只负责格式解析（bytes → DataTable），不包含数据处理。但 polars 组件导出了完整的 DataFrame 操作能力（select/filter/sort/group-by/join）。这是有意为之的方向 B 决策——Polars 的核心价值在于 DataFrame 操作而非仅仅是解析。这意味着组件职责边界可以按需扩展，不必拘泥于"仅解析"原则。

## Architecture Insight: WASM 单线程下 Lazy API 无优势

Polars Lazy API 的主要优势是查询优化（谓词下推 + 自动并行）。但在 WASM 单线程环境下，并行优化本身无效。因此使用 Eager API 在功能完整性和性能上均无实际损失。如果 WASI 后续版本引入线程支持，应重新评估。

## Architecture Insight: 宿主多实例并行模式

WASI 0.3 无原生多线程，但宿主可通过数据分片 + 多 Component 实例实现并行。具体做法：将大数据集分成 N 份，创建 N 个 polars Component 实例，各自独立处理分片，最后由宿主合并结果。这适合 map-reduce 风格的工作负载。

## Future TODO: Image encode-image 改为 borrow 语义

当前 `encode-image` 按值接收 `image-handle`（WIT 语义下 handle 被 consume），调用后宿主无法再使用该 handle。如需多次编码不同格式，需要 re-decode。考虑将 WIT 签名改为 `func(img: borrow<image-handle>, ...)` 以支持非消耗式编码。但这需要 WIT borrow 语义支持，需验证 wasi-component-model 当前版本的 borrow 支持程度。

## Future TODO: Image AVIF 格式支持

avif 格式依赖 `ravif` crate，编译后体积较大且编码慢。如需支持 AVIF 解码/编码，添加 `"avif"` feature 到 image 依赖即可。需评估 WASM 体积增量。

## Future TODO: Image 组件体积优化

limpuai-wit-image 编译到 WASM 后约 3.2 MB。可通过裁剪未使用的图像格式 feature、使用 wasm-opt/wasm-strip 后处理来减小体积。如果只需要常用格式（PNG/JPEG/WebP），可去掉 exr/hdr/ff/pnm/tga/qoi 等小众格式。

## Future TODO: Image cargo about 更新 THIRD-PARTY-LICENSES

image 组件引入了新的依赖树（含 image 及其子 crate：zune-jpeg, png, tiff, image-webp, gif 等），需运行 `cargo about` 更新 THIRD-PARTY-LICENSES.md 文件。

## Future TODO: Image 更多测试覆盖

当前测试覆盖 PNG 和 JPEG 解码、基本操作和编码。缺少：rotate180/rotate270 独立测试、BMP/TIFF/WebP/GIF 编码测试、WebP/TIFF/BMP/GIF 解码测试、多操作链式调用测试、crop 越界边界测试。

## Controversy: Image invert 语义

`invert` 设计为原地修改原图（与 image-rs 行为一致），但 WIT 签名要求返回 `result<image-handle, string>`。实现上同时做了原地修改和返回新 handle（克隆修改后的图像）。这是 WIT 接口约束和设计意图之间的妥协。更好的方案是将 WIT 签名改为 `invert: func() -> result<(), string>`（无返回值），但这与其他操作的风格不一致。

## Architecture Insight: encode-image consume 模式适用于 fire-and-forget

`encode-image` 的 consume 语义在某些场景下是合理的：解码 → 操作链 → 编码输出（一次性流水线）。但对于需要多次编码的场景（如图像转换服务），需要多次 decode 或在宿主端缓存原始字节。这反映了 WIT Resource 的所有权模型设计选择。
