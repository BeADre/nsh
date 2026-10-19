---
title: 方案对比
description: Native Shell 如何组合 AI 生成界面、运行时交付和宿主能力，以及它与其他跨平台 UI 框架的方向差异。
order: 2
group: start
---

# 方案对比

Native Shell（NSH）是面向 **AI 生成界面、动态分发和宿主内组合**的跨平台 UI 引擎。应用可以加载发布时尚未写好的界面：按任务生成的表单、可操作的结果、合作方模块或用户安装的扩展。模块拥有状态和交互逻辑，宿主决定它能够使用哪些组件、服务与资源。

NSH 也支持构建完整的手机和桌面应用。动态 TSX 模块、预编译 Rust 组件与宿主原生视图可以共同使用，也可以构建不链接 QuickJS 的[纯 Rust 应用](./rust.md)。动态交付与编译后的核心功能可以存在于同一个应用中。

## 从任务到交互界面

AI 助手的一次回答可能需要可筛选列表，另一次需要分步确认。NSH 为这些界面提供从生成到执行的路径：

- **表达交互：**类型化 TSX API 覆盖组件、状态、事件和异步操作。[无界面检查与 trace](./testing-and-trace.md) 帮助在集成前检查生成的界面。
- **交付模块：**加载 bundle 或验证[签名包](./signed-packages.md)，执行运行时兼容和版本检查。下载服务由宿主提供，何时接受更新也由宿主决定。
- **组合应用：**将 Surface 嵌入已有的 [iOS](./embedding-ios.md)、[Android](./embedding-android.md) 或 [GPUI 应用](./gpui.md)，通过宿主开放[原生组件](./native-components.md)和 Rust 业务 API。
- **设定执行边界：**原生动态 Surface 使用独立 VM、资源预算和明确的能力策略；业务操作的授权仍由宿主负责。

这里交付的是界面代码及其行为，超出了为已有页面更新数据或开发时热重载的范围。新增原生二进制能力仍需构建并发布宿主应用；分发也须遵守目标平台的政策。

## 其他动态界面路径

| 方案 | 运行时交付什么 | 宿主管理什么 |
| --- | --- | --- |
| **WebView** | HTML、CSS 与 JavaScript，由浏览器布局和渲染 | 网页与原生集成、导航及原生服务访问 |
| **服务端驱动 UI** | 组件与动作 Schema；表达式和状态支持取决于协议 | 组件目录、协议兼容和允许的行为 |
| **应用代码更新** | 与已安装运行时兼容的代码和资源 | 更新选择、原生兼容和应用的信任模型 |
| **NSH 模块** | 可编程 TSX 界面，组合宿主注册的能力 | 模块验证、Surface 生命周期、执行预算和能力策略 |

这些方式可以共同使用。网页内容适合 WebView，可预测的表单可能只需一套 Schema。NSH 关注的是拥有自身交互逻辑，同时复用宿主原生与 Rust 能力的模块。

## 框架的方向差异

这里比较的是产品适用方向，不是性能排名。这些框架都可以扩展，其中多个已经支持嵌入和动态加载。

| 框架 | 主要方向 | 动态界面或嵌入路径 |
| --- | --- | --- |
| **NSH** | 运行时交付界面与宿主组合，同时提供 Rust 和 TSX 应用路径 | Surface 加载、签名包、宿主能力和执行限制 |
| **React Native** | React 驱动的原生应用与原生扩展 | 嵌入单个视图或流程；通过更新服务交付兼容的 JS 与资源 |
| **Flutter** | 跨平台统一的 Widget、动画与渲染体系 | Add-to-app、平台视图、服务端驱动 Widget 与第三方代码更新 |
| **GPUI** | 以 Rust 代码构建 GPU 加速 UI，主要用于桌面 | 应用或附加运行时提供动态模块；上游已有浏览器路径 |
| **Qt Quick / Qt Widgets** | QML 与场景图 / 传统桌面控件工具箱 | 动态加载和创建 QML 组件；通过应用代码构造控件 |
| **Slint** | 声明式 UI，连接 Rust、C++ 等业务逻辑 | 编译 UI，或通过解释器加载 |
| **MyGo** | Go 原生 GPU UI 或系统 WebView 前端 | 原生 Go UI、网页内容，以及原生 UI 的 WebAssembly 浏览器目标 |

### React Native 与 Flutter

React Native 支持[逐步嵌入已有应用](https://reactnative.dev/docs/integration-with-existing-apps)，包括单个视图或业务流程。[EAS Update](https://docs.expo.dev/eas-update/introduction/) 可以向兼容运行时交付 JavaScript、图片等非原生资源；改变原生能力则需要新的二进制。NSH 将独立管理的 Surface 放在这条路径的中心，并提供直接用 Rust 实现组件和业务逻辑的方式。

Flutter 的[架构](https://docs.flutter.dev/resources/architectural-overview)提供统一的 Widget 与绘制模型，也有 Add-to-app 和平台视图用于原生集成。[官方 FAQ](https://docs.flutter.dev/resources/faq#does-flutter-support-code-push)区分了第三方 code push 与框架内置能力。若产品的核心需求是承载运行时交付的模块，并与已有宿主服务组合，NSH 是一种选择。已有 React 或 Flutter 团队也应结合组件库、工具积累与集成成本来判断。

### GPUI

[GPUI](https://www.gpui.rs/) 提供 Rust UI 基础，GPUI Kit 在这一生态上提供可复用组件。NSH 可以为已有桌面产品提供动态界面层。其 [GPUI 后端](./gpui.md)目前基于 GPUI Fast，将 NSH Surface 作为 GPUI 视图嵌入。主体应用和性能敏感组件继续使用 Rust，再开放模块需要的能力。NSH 为手机客户端提供独立的平台后端。上游 GPUI 也已有[浏览器示例](https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/README.md)。

### Qt Quick 与 Qt Widgets

Qt Quick 已支持[动态创建 QML 组件](https://doc.qt.io/qt-6/qtqml-javascript-dynamicobjectcreation.html)，[Qt Widgets](https://doc.qt.io/qt-6/qtwidgets-index.html) 则提供成熟的桌面工具箱，两者都能连接原生业务。NSH 更强调独立加载模块的交付与管理：包的来源、宿主能力、资源限制和 Surface 生命周期。依赖 Qt 成熟控件与平台模块的团队，应将这些积累纳入选择。

### Slint 与 MyGo

除了常用的编译路径，[Slint 解释器](https://docs.slint.dev/latest/docs/rust/slint_interpreter/)也支持运行时加载 UI。NSH 使用 TSX 模块表达状态、异步操作和交互，并通过包与宿主策略管理执行。Slint 同样支持状态和命令式事件处理；这里的差异在于 NSH 的包与宿主能力管理流程，而不是界面能否包含逻辑。

[MyGo](https://mygo.egoist.dev/docs/architecture) 为 Go 开发者同时提供原生 UI 与系统 WebView 路径，原生 UI 也有 WebAssembly 浏览器目标。NSH 更关注手机和桌面的模块承载，以及 Rust 集成。Go 桌面产品可能更适合 MyGo 的语言和工具；Rust 或 GPUI 宿主若需增加动态任务界面，NSH 提供了相应的组件与运行时模型。

## 组合方式与性能

可以从现有应用中的一个面板开始，增加动态交付的页面，也可以构建完整客户端。图表、编辑器或业务服务保留为编译后的 Rust 实现，由 TSX 模块组织控件和交互。宿主注册希望开放的能力，模块本身不会引入新的原生二进制。

[纯 Rust 与混合应用](./rust.md)使用相同的核心组件和响应式模型。纯 Rust 可以移除 QuickJS 依赖，混合应用为动态 TSX 保留它。这是一种实现选择，不代表界面必然更快：布局、文本、绘制、业务逻辑和平台集成仍共同决定最终性能。

## 平台路径

| 目标 | NSH 的呈现路径 |
| --- | --- |
| iOS | UIKit 与 CoreText |
| Android | Android Views 与平台文本、输入集成 |
| macOS | AppKit、Core Animation 与 CoreText |
| Windows | 在 Win32 中用 Direct2D、DirectWrite 绘制保留视图树 |
| Linux | GTK 4、自定义 GSK 绘制与 Pango 文本 |
| GPUI | 通过 GPUI 后端绘制 NSH 视图树 |
| Web | 在浏览器中通过 HTML 与 DOM 呈现 |

共享组件不意味着各端行为完全相同，也不意味着每个控件都是操作系统原生控件。请按应用需要查阅[平台指南](./platforms.md)。

在 [Web](./web.md) 上，Rust 和 TSX 应用都呈现为 HTML。动态 TSX 使用 QuickJS，纯 Rust 应用可以移除这一依赖。浏览器中的呈现方式与应用语言的选择是两个不同维度。

## 执行边界

[签名包](./signed-packages.md)通过信任根、发布证书和文件哈希验证来源与完整性；撤销、最低运行时版本和回滚保护控制是否接受包。这些机制不保证模块的行为安全。

[原生 Surface 限制与能力策略](./sandbox-and-capabilities.md)覆盖 VM 内存、栈、调用时间、节点与排队操作，以及网络目标、WebView 导航、剪贴板等受管理能力。需要约束模块时应明确配置策略：省略声明意味着模块层面不设限制，有效访问仍须满足宿主的授权。

VM 是进程内的执行边界，不等同于操作系统进程隔离。自定义原生和业务 API 必须自行检查授权；QuickJS 字节码必须来自可信输入，例如已验证的包。决定接受哪些生成代码时，需要评估完整宿主及其开放的能力。

## 何时选择 NSH

AI 助手需要生成可操作的结果、客户端需要独立交付模块，或已有原生与 Rust 应用需要动态扩展层时，NSH 尤其适用。核心选择是让界面在运行时到来，同时由宿主控制其集成和执行方式。不需要动态代码时，也可以用它构建完整的 Rust 应用。
