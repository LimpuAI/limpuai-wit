# polars-component Requirements

## What we need

将 Polars Rust 库编译为 WASI Component，提供完整的 DataFrame 操作能力（方向 B：不仅解析，还导出数据操作）。

Polars 的核心价值在于高性能 DataFrame 操作（筛选、聚合、排序、连接等），而非仅仅是文件解析。本项目将其能力通过 WIT 接口导出，使任何 WASI 运行时无需绑定原生库即可使用。

## 一期范围

### 读取能力
- CSV 文件解析（依赖 `polars` csv feature）
- JSON / NDJSON 文件解析（依赖 `polars` json feature）

### DataFrame 操作（Eager API）
- **选择**：`select` — 按列名选取子集
- **筛选**：`filter` — 按条件过滤行（支持比较、逻辑运算）
- **排序**：`sort` — 按列排序（升序/降序）
- **截取**：`head` / `tail` — 取前/后 N 行
- **去重**：`unique` — 去重
- **聚合**：`group-by` + 聚合函数（sum, mean, min, max, count）
- **连接**：`join` — 两个 DataFrame 之间的 inner/left/outer/full join

### 导出
- 将 DataFrame 转换为 `DataTable`（复用现有共享类型）

## Input & Output

**Input**:
- CSV/JSON 文件原始字节 (`list<u8>`)
- DataFrame 操作指令（通过 WIT 接口方法调用）

**Output**:
- 转换后的 `DataTable`（共享类型 `data-table`）

## Success criteria

- [x] WIT 接口定义：polars-parser.wit（包含 resource dataframe + 工厂函数）
- [x] `limpuai-wit-polars` crate 依赖 crates.io `polars`（非本地 path）
- [x] 编译到 wasm32-wasip2（stable Rust，无需 nightly）
- [x] CSV 解析正确，输出 DataTable
- [x] JSON/NDJSON 解析正确，输出 DataTable
- [x] DataFrame 操作（select/filter/sort/group-by/join）通过 WIT resource 暴露
- [x] 操作链可用：parse → transform → export DataTable
- [x] 测试覆盖核心操作
- [ ] 使用 `cargo about` 生成 THIRD-PARTY-LICENSES 更新

## Edge cases

- 无效 CSV/JSON 数据：返回明确错误信息
- 空 DataFrame：不崩溃，返回空 DataTable
- 不支持的类型（List/Struct/Map）：映射为 `null`（与 arrow/parquet 组件一致）
- 非法操作（select 不存在的列、join key 不匹配）：返回错误字符串
- 大数据集：单线程执行（WASM 无 rayon 线程池，Polars 已有 cfg fallback）

## Known limitations

- **无 Lazy API**：`lazy` feature 依赖 `ring` crate（C 编译），无法在 wasm32-wasip2 编译
  - 影响：无查询优化器（谓词下推、投影下推、CSE 消除）
  - 不影响：WASM 本身是单线程环境，lazy 的自动并行在 WASM 里也无效；eager API 功能完整
- **无 Parquet/IPC 写入**：polars-parquet 的压缩依赖需要原生 C 编译
- **单线程**：Polars 在 WASM 目标上自动降级为单线程（cfg 条件编译已内置）
- **WASM 体积较大**：编译后 ~26 MB，可通过裁剪 features 和 wasm-opt 优化

## might-it-be.md 关联

- **World 命名规范**：应统一命名风格，新组件建议使用 `polars-dataframe` 作为 world 名
- **组件职责边界**：解析 + 操作能力在同一个组件内，因为 Polars DataFrame 是统一抽象
- **嵌套类型**：沿用现有策略，List/Map/Struct → `null`
