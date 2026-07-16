# Atlas

Atlas 是一个面向软件开发与计算机科学研究的 Agent 桌面 IDE。项目基于 Rust、Wry/WebView2 和普通 Web 前端构建，将对话、代码工作区、领域研究环境、可视化与证据管理整合在同一个桌面应用中。

仓库地址：<https://github.com/silverenternal/try-tokitai>

## 核心能力

- Agent 对话：流式响应、工具调用、文件修改、终端、浏览器与 Git 工作流。
- 研究模式：面向开放式计算机科学问题的规划、实验、验证、审查与论文产出。
- 领域工作台：覆盖 AI/ML、计算机视觉、NLP、图形学、CAD、机器人、网络、操作系统、编译器、数据库、软件工程、程序分析、网络安全、HPC、分布式系统和科学计算。
- Research OS：统一管理假设、实验谱系、证据、负结果、决策、研究记忆、时间线和发表物。
- 交互式可视化：图、时间序列、性能数据、领域文档和 3D 几何预览。
- 多种宿主：桌面端、CLI、TUI、Web 与 MCP Server。

领域任务与 Agent 使用共享状态。Agent 可以读取当前领域、选中对象、参数、视图、笔记、真实工作区文件和可用 SDK；完成任务必须提交工作区内的真实产物与验证证据。任务状态、产物、验证命令和实验谱系会同步到 Research OS。

## 快速启动

### 环境要求

- Rust stable 与 Cargo
- Windows 10/11 桌面端需要 Microsoft Edge WebView2 Runtime（通常已预装）
- 至少一个受支持的模型 API，或本地 Ollama

复制环境变量模板并填写所需配置：

~~~powershell
Copy-Item .env.example .env
~~~

### 桌面应用

开发构建：

~~~powershell
cargo run --bin desktop_wry --features desktop-shell
~~~

发布构建：

~~~powershell
cargo build --release --bin desktop_wry --features desktop-shell
.\target\release\desktop_wry.exe
~~~

生成 Windows 便携包：

~~~powershell
.\scripts\package-desktop.ps1
~~~

发布说明参见 [DESKTOP_RELEASE.md](DESKTOP_RELEASE.md)。

### CLI、TUI 与 MCP

~~~powershell
# CLI
cargo run --release

# TUI
cargo run --release -- --tui

# MCP Server
cargo run --release -- --mcp

# Web
cargo run --release -- --web
~~~

## Research OS

Research OS 不是单独的静态面板，而是 Agent 与领域工作台共享的研究账本：

- 领域任务自动进入实验谱系。
- 产物与验证项分别记录，保留来源路径与复现命令。
- 失败任务进入负结果库，避免重复走已经失败的路线。
- Agent 可通过 research_os_snapshot 查询研究状态，通过 research_os_mutate 更新研究对象。
- 没有关联证据时，假设不能标记为已验证或已证伪，发表物也不能标记为可发布。

使用说明参见 [RESEARCH_OS_USER_GUIDE.md](RESEARCH_OS_USER_GUIDE.md)。

## 领域工作台

右侧领域栏用于打开不同的专业环境。工作台与对话区可以左右共存，Agent 在执行期间会保留领域上下文。任务完成后生成 Research Preview 卡片；点击卡片会打开对应领域并定位结果或可视化。

领域原生操作依赖本机工具链。例如 Wireshark/tshark、Clang、SQLite、Blender、FreeCAD、ROS 2、Semgrep、Kubernetes、NumPy、SciPy 或 VTK。未检测到依赖时，界面会显示不可用原因，不会伪造执行结果。

## 验证

~~~powershell
cargo test
node tools/test_research_domains_wiring.mjs
node tools/test_research_os_wiring.mjs
node tools/test_visualization_wiring.mjs
~~~

针对桌面端的发布构建：

~~~powershell
cargo build --release --bin desktop_wry --features desktop-shell
~~~

## 目录

~~~text
frontend/              桌面/Web 前端
src/                   Rust 应用与 Agent 后端
src/research_domains/  领域注册、任务、动作与状态
src/research_os/       研究对象、证据与谱系
crates/                工作区内部 crates
tools/                 接线与回归检查脚本
scripts/               构建、打包与维护脚本
docs/                  当前架构和使用文档
~~~

## 安全与数据

- .env、本地 Research OS 状态、临时截图、日志和构建产物不会提交到 Git。
- 领域任务只接受工作区内的相对路径；完成状态要求真实文件和验证证据。
- 外部命令通过受控工具链与安全策略执行。

## 许可证

MIT OR Apache-2.0
