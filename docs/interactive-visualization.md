# Interactive Visualization

Atlas 将跨领域的科研产物交给 Research Domains Workspace。Interactive Visualization 只保留不属于单一领域工作台的全局视图：

- `System`：本机 CPU、GPU、内存、磁盘、进程与网络设备的真实运行状态；
- `Paper`：论文结构、章节与引用关系；
- `Multi-Agent`：当前持久会话、Agent、工具调用与运行时间线。

`Algorithm` 与 `Network` 不再注册为 Interactive Visualization 类型。算法、模型、训练和网络产物分别由对应 Research Domain Workspace 的专业信息架构、工具栏和 Preview Card 承载。

## 数据边界

`VisualizationSource` 描述真实输入来源，`VisualizationAdapter` 将来源解析为统一的 `atlas.visualization.v1` 文档，前端通用渲染器只读取该文档。统一文档包含 `nodes`、`edges`、`series`、`events`、`frames` 和 `diagnostics`。

Adapter 不得注入演示节点、固定流程或模拟指标。数据为空或平台不支持采集时，应返回空文档和明确诊断。

## API

- `GET /api/visualizations`：返回 System、Paper、Multi-Agent 类型及其真实数据源；
- `GET /api/visualizations/snapshot?kind=...&source_id=...`：返回统一文档快照。

Agent 生成的可视化科研产物不经过 Algorithm/Network 回退卡片。除 Paper 与 Multi-Agent 的专属卡片外，最终输出卡片均通过 `atlas:research-domain-open` 打开对应领域，定位并高亮真实产物。
