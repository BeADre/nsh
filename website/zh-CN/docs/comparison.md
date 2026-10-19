---
title: 方案对比
description: 从 AI 生成界面、动态分发、宿主组合到完整 Rust 应用，了解 Native Shell 与 React Native、Flutter、GPUI、Qt 和 Slint 的方向差异与适用场景。
order: 2
group: start
---

# Native Shell 与跨平台 UI 框架

Native Shell（NSH）是为 **AI 生成界面和动态分发**设计的跨平台 UI 引擎。应用可以在运行时加载新的交互界面，将它与现有页面、业务服务和原生组件组合，并由宿主管理代码来源、权限、资源限制与生命周期。

我们的设计起点，是让手机和桌面应用能够承载发布时尚未写好的界面：AI 根据任务生成的交互面板、按需交付的业务页面、合作方提供的模块，以及用户安装的扩展。这些界面可以拥有自己的状态、逻辑和交互，并使用宿主开放的原生能力。

在此基础上，NSH 也可以构建完整的手机与桌面客户端。应用可以选择动态模块、预编译 Rust 组件与平台原生界面的组合方式；性能敏感的 UI 与业务可以直接用 Rust 实现，纯 Rust 应用可以完全不链接 QuickJS。

## 我们首先解决什么问题

传统应用通常在发布前确定页面与功能。AI 带来的变化是：用户提出任务后，应用才知道最适合的交互形式。一次结果可能需要表单，另一次需要可筛选的数据列表、图表、步骤确认或可操作的详情页。

NSH 为运行时生成和交付界面提供一条完整的使用路径：

| 核心需求 | NSH 提供的能力 | 对应用的价值 |
| --- | --- | --- |
| AI 生成交互界面 | 类型化组件与 API；带状态、逻辑和事件处理的 TSX 模块 | AI 的结果可以成为用户能够操作的界面 |
| 运行时交付新界面 | 动态加载、签名包验证、运行时兼容与版本检查 | 页面和模块可以独立于宿主二进制交付 |
| 与现有应用组合 | 嵌入原生宿主，调用已注册的组件与业务 API | 在已有产品中逐步增加动态界面 |
| 管理动态代码的边界 | 原生动态 Surface 的独立 VM、执行预算和宿主能力策略 | 分别管理模块与宿主之间的信任和访问边界 |
| 进入手机与桌面客户端 | 平台后端、共享组件抽象与各端交互适配 | 相同的动态界面模型可以服务不同客户端 |
| 保留编译后的实现路径 | Rust 组件、业务模块与完整 Rust 应用 | 核心功能可以使用编译后的代码，动态模块负责组合 |

这里的动态交付，是在应用运行时加载新的界面模块及其行为。它比根据数据切换已有控件更进一步，也区别于开发环境中的热重载。宿主负责模块下载与更新时机，NSH 负责加载、验证和执行；新增原生二进制能力仍通过宿主的构建与发布流程交付。应用的分发方式也须遵守目标平台的政策。

## 与常见动态界面方案的区别

WebView、服务端驱动 UI 和应用代码更新都能服务动态内容。NSH 将可编程交互、原生宿主组合和模块执行管理放在同一条路径中。

| 方案 | 界面的表达方式 | 应用重点管理的部分 | NSH 提供的选择 |
| --- | --- | --- | --- |
| WebView 页面 | HTML、CSS 与 JavaScript，由浏览器布局和呈现 | 网页与原生能力的接口、导航、交互协调与授权 | 通过平台后端呈现模块，调用宿主注册的组件与服务 |
| 服务端驱动 UI | 宿主支持的 Schema、组件和动作协议；状态及表达式能力取决于协议 | 组件目录、协议版本与行为范围 | 用可编程模块组织状态、异步操作和交互，复用宿主能力 |
| 应用代码更新 | 更新与已安装运行时兼容的代码和资源 | 更新兼容性、受信任代码范围及应用发布模型 | 以 Surface 为单元管理界面模块的执行与生命周期 |
| NSH 动态模块 | TSX 模块与宿主组件共同组成交互 | 可用组件、业务接口、来源、权限和资源预算 | 适合按用户任务生成并交付可操作的界面 |

这些方案可以共同使用。文档或已有网页适合放进 WebView；稳定、可预测的表单可能只需一套 Schema。NSH 的侧重点是：当任务需要新的交互逻辑时，模块仍能复用客户端已有能力，并接受宿主的执行管理。

## 动态界面可以怎样组合

NSH 可以承担应用中的一个区域、一张页面、一个扩展，也可以承担完整界面。宿主决定开放哪些能力，模块使用这些能力完成具体任务。

| 组合方式 | 示例 | NSH 的作用 |
| --- | --- | --- |
| 原生客户端加 AI 动态界面 | 聊天结果中出现筛选表单、结果列表与操作面板 | 把一次 AI 任务变为可交互的客户端界面 |
| 已有业务页面加动态模块 | 订单详情中加载按场景交付的工具或解释面板 | 扩展局部功能，复用既有页面结构与服务 |
| Rust 组件加动态页面 | 编译后的图表、编辑器或数据处理能力，由 TSX 组织交互 | 保留核心实现，灵活交付页面组合 |
| GPUI 客户端加动态扩展 | 在 GPUI 或 GPUI Kit 应用中嵌入 NSH Surface | 为已有 Rust 桌面产品增加动态界面层 |
| 稳定核心加可选插件 | 主体预编译发布，部分工具或工作流按需加载 | 让核心功能与扩展模块按各自节奏演进 |
| 独立客户端 | 用 Rust、TSX 或两者混合构建完整应用 | 同一套核心组件能力服务动态模块与完整应用 |

[iOS](./embedding-ios.md) 与 [Android](./embedding-android.md) 的嵌入接口负责接入原生宿主；[GPUI 后端](./gpui.md)提供桌面 Rust 接入路径。[原生组件注册](./native-components.md)让宿主提供组件工厂、属性、事件、命令与尺寸协作。组件的具体实现和业务授权仍由宿主负责，动态模块只能使用宿主已经提供的能力。

## 为生成代码建立执行边界

AI 生成界面需要的不只是渲染器。应用还需要知道代码来自哪里、可以访问什么，以及异常模块会占用多少资源。NSH 将这些问题纳入模块的交付和执行体系。

| 边界 | NSH 的机制 | 宿主管理什么 |
| --- | --- | --- |
| 来源与完整性 | 签名包、信任根、发布证书与文件校验 | 接受哪些发布方交付的模块 |
| 版本与兼容 | 撤销、最低运行时版本与回滚保护 | 接受的模块版本及运行时要求 |
| 执行环境 | 原生动态 Surface 使用独立 QuickJS VM | 各 Surface 的脚本状态与生命周期 |
| 执行资源 | 堆、栈、调用时间、节点与排队操作等限制 | 模块的部分执行与界面资源预算 |
| 平台能力 | 模块声明能力，宿主可进一步收紧 | 网络目标、WebView 导航、剪贴板等受管理访问 |
| 业务能力 | 宿主注册组件和业务 API，并实施业务授权 | 模块可以执行哪些产品操作 |

[签名包](./signed-packages.md)用于验证来源与完整性；[运行时限制和能力策略](./sandbox-and-capabilities.md)用于约束执行行为。需要限制模块时应明确配置策略：省略能力声明意味着模块声明这一层不额外收紧能力；有效访问仍须满足宿主授权，资源限额独立生效。

宿主开放的自定义原生和业务 API 必须自行检查授权。VM 是进程内的边界，不等同于操作系统进程隔离；签名也不保证行为安全。QuickJS 字节码必须来自可信输入，例如已验证的包。应结合宿主及其开放能力评估完整的信任边界。撤销列表需要宿主获取并更新到信任存储，在打开包时检查；已运行的模块何时停止或重载仍由宿主管理，签名撤销不是即时远程终止机制。

## 用同样的任务看差异

下文用三个任务比较框架实际提供什么，以及宿主还需要实现什么：

1. **生成一个新交互：**AI 在应用发布后生成订单确认表单，包含校验、异步查询和确认逻辑，调用宿主已经提供的保存订单 API。
2. **管理合作方模块：**同时加载两个来源不同的模块，为它们设置不同的网络权限和执行预算，并能拒绝已经撤销的包。
3. **复用 Rust 核心：**保留编译后的图表或计算服务，让动态界面控制参数、显示结果，并正确处理组件销毁与异步回调。

这三件事分别检验表达和交付、执行治理、原生集成。拥有动态语言不自动带来模块授权；能够调用 Rust 也不代表已有 UI 可以零成本嵌入。

对于动态 TSX，NSH 在原生端按 Surface 分配 QuickJS VM 和引擎线程，信号图、组件核心与布局由 Rust 实现，一次提交形成一个挂载批次。Web 使用页面线程上的本地引擎，不能照搬原生端的线程隔离结论。VM 的调用时间与堆限制也不覆盖任意宿主原生函数消耗的全部 CPU 或内存；宿主 API 需要自己的预算和取消设计。机制见[架构](./architecture.md)与[执行限制](./sandbox-and-capabilities.md)。

## 与其他框架的方向差异

以下以 2026 年 10 月 19 日的实现与官方文档为依据，比较各框架提供的机制及宿主需要完成的工作。NSH 的侧重点，是将生成、交付、组合和执行边界组织成一条模块使用路径；性能仍应在具体产品上测量。

| 框架 | 主要设计重心 | 动态界面的路径 | NSH 更关注的部分 |
| --- | --- | --- | --- |
| **NSH** | AI 生成界面、动态分发和宿主组合，也支持完整应用 | 模块加载、签名包与宿主执行管理 | 将界面模块的交付和执行作为一套能力提供 |
| React Native | 用 React 开发原生应用与原生扩展 | JS 模块、更新服务与原生组件 | 独立 Surface 的管理，以及 Rust 核心与动态模块的组合 |
| Flutter | 统一的 Widget、动画和绘制体系 | 运行时组合 Widget、服务端驱动 UI、第三方更新 | 在宿主中承载按任务交付的界面模块 |
| GPUI | Rust 代码驱动的 GPU UI，主要用于桌面，上游也有浏览器路径 | 应用或上层运行时提供动态模块 | 为已有 Rust 客户端增加动态层，并提供移动后端 |
| Qt Quick | QML、属性绑定与场景图 | 动态加载和创建 QML 组件 | 围绕模块来源、权限和交付组织使用方式 |
| Qt Widgets | 成熟的桌面控件与应用工具箱 | 通过应用代码动态创建控件 | 面向手机和桌面的可交付交互模块 |
| Slint | 可编译的声明式 UI，连接 Rust、C++ 等业务 | 编译 UI，或用解释器运行时加载 | TSX 模块与宿主能力、签名交付和资源预算的组合 |

### 与 React Native 相比：同样能交付 JS，模块边界放在哪里

React Native 也能嵌入已有应用，运行新交付的 JavaScript，并调用已安装的原生能力。NSH 将模块的执行、能力和交付边界集中在 Surface 上；使用 RN 时，宿主从 React 运行时与更新服务出发组织相应策略。

| 维度 | NSH 的实际机制 | React Native 的实际机制 | 实际差异与代价 |
| --- | --- | --- | --- |
| 运行时与调度 | 原生动态 Surface 各有 QuickJS VM 和引擎线程，提交批次到 UI 线程 | 通常使用 [Hermes](https://reactnative.dev/docs/hermes)；新架构通过 [JSI 与原生协作](https://reactnative.dev/blog/2024/10/23/the-new-architecture-is-here)，React 调度可区分更新优先级 | 都可以让 JS 驱动原生 UI；NSH 把 VM 作为 Surface 边界，RN 的 React 调度不等于限制任意脚本执行时间 |
| 状态、布局与挂载 | Rust 信号图追踪依赖，组件初始化后更新绑定；Taffy 布局，提交 MountBatch | [Fabric](https://reactnative.dev/architecture/render-pipeline) 由 React 更新 C++ 影子树，Yoga 布局，再 diff 并挂载宿主视图 | 两者都有原生布局和批量挂载；NSH 是信号模型而非 React 重渲染模型，React Hooks 与组件不能只换导入就迁移 |
| Surface 与运行时关系 | 一个原生动态 Surface 对应一个 VM、状态和执行预算 | [ReactHost](https://github.com/react/react-native/blob/main/packages/react-native/ReactAndroid/src/main/java/com/facebook/react/ReactHost.kt) 管理 ReactInstance 与多个 Surface；Surface 本身不意味着独立 VM | NSH 的模块预算有明确归属；RN 可以另建运行时，但宿主需规划共享模块、状态与销毁关系 |
| 新交互的交付 | bundle 或签名包包含模块行为，使用宿主已有 API | [EAS Update](https://docs.expo.dev/eas-update/runtime-versions/) 交付与 native runtime version 匹配的 JS 和资源 | 两者都能更新已有原生能力之上的逻辑；新增原生 API 都需要宿主升级。应用更新版本与独立合作方模块版本需要分别设计 |
| 签名与模块策略 | 包验证、撤销和回滚检查结合 Surface 能力交集与 VM 限额 | EAS 也提供[更新签名](https://docs.expo.dev/eas-update/code-signing/)；签名更新不自动带有每个 Surface 的权限和预算 | NSH 的差异是验证后如何受限运行，而不是独占签名能力；RN 宿主要补充模块级授权、撤销和资源策略 |
| 原生 UI 与 Rust | 宿主注册组件的属性、事件和命令；Rust 组件和业务类型可向 TSX 开放 | [Fabric 原生组件](https://reactnative.dev/docs/fabric-native-components-introduction)用类型规范和 Codegen 接入；[C++ TurboModule](https://reactnative.dev/docs/the-new-architecture/pure-cxx-modules)可作为 Rust ABI 的适配入口 | Rust 计算两边都能复用；NSH 的 Rust 核心减少跨语言实现层，RN 仍需 Rust/C++ 或平台适配及对象生命周期管理。已有原生视图两边都需接入 |
| 平台与已有代码 | 各端 NSH 后端共享组件模型，Web 呈现 HTML | RN 核心面向 iOS/Android，Web 等由[生态平台实现](https://reactnative.dev/docs/next/out-of-tree-platforms) | NSH 提供统一模块路径；已有 RN 组件库、Hooks 与原生依赖仍是 RN 产品的重要资产，迁移需逐项核对 |

**同样生成订单确认界面：**RN 可以通过更新机制交付新的 JS，调用已有保存订单模块；NSH 可以交付一个带该交互的 Surface 包。若只更新自己的受信任应用，RN 的既有更新链路已经贴合需求。若同屏承载多个来源、权限不同的任务模块，NSH 提供更直接的预算和策略挂载点；RN 需要把这些规则加到运行时和宿主接口上。

**同样撤销合作方模块：**两者都需要宿主取得最新策略，并决定何时卸载已运行界面。NSH 已定义包撤销检查与 Surface 预算；RN 的运行时/Surface 生命周期提供基础，但不是同一套包治理协议。多实例也不是 NSH 独有：[Expo SDK 56](https://expo.dev/changelog/sdk-56)已提供实验性的 iOS 多应用嵌入路径，应按所选版本评估。

### 与 Flutter 相比：应用补丁、远程 Widget，还是可执行任务模块

Flutter 的 Widget、布局和绘制体系适合完整跨平台应用；NSH 更关注把新交互模块放进已有宿主。Flutter 可以结合更新服务、远程 Widget 或额外执行引擎；NSH 将可执行任务模块作为现成单元，两边需要宿主补齐的能力因而不同。

| 维度 | NSH 的实际机制 | Flutter 的实际机制 | 实际差异与代价 |
| --- | --- | --- | --- |
| 运行时与调度 | 原生 Surface 使用 QuickJS 引擎线程；Rust 核心提交挂载批次 | 原生发布使用 AOT Dart；当前原生平台默认采用[合并的 UI/平台线程](https://docs.flutter.dev/release/breaking-changes/linux-merged-threads)，光栅化另行执行，计算可用 [isolate](https://docs.flutter.dev/perf/isolates) | NSH 分离模块脚本与挂载；Flutter 已是编译后的执行路径，不应把选择 Rust 写成对 Dart 的速度结论 |
| 状态与布局 | 信号更新绑定，核心树用 Taffy 布局 | [Widget 构建、Element 身份与 RenderObject 布局/绘制](https://docs.flutter.dev/resources/architectural-overview)协作，按父约束与子尺寸布局 | 两边的组件模型和布局约定不同；Flutter Widget 不能直接当作 NSH 组件使用 |
| 控件与呈现 | 移动端平台视图，桌面按后端绘制，Web 为 HTML | 通常由 Flutter 引擎绘制控件；平台视图用于嵌入宿主控件 | Flutter 提供统一绘制和动画体系；NSH 更贴近各宿主的呈现路径，但各端行为仍需验证 |
| 多界面与生命周期 | 每个原生动态 Surface 有自己的 VM、生命周期与预算 | [FlutterEngineGroup](https://docs.flutter.dev/add-to-app/multiple-flutters)支持独立状态的多引擎，并共享可复用资源 | 两者都能局部嵌入和多实例；独立状态本身不定义合作方包权限，NSH 额外把执行策略纳入 Surface |
| 新交互的交付 | TSX 包同时交付布局、状态、异步逻辑和事件 | [RFW](https://pub.dev/packages/rfw)下载 Widget 描述与数据，并调用宿主提供的事件；可再结合脚本机制。[Shorebird](https://docs.shorebird.dev/code-push/)提供 Dart 代码补丁 | RFW 的回调范围、应用补丁与独立任务模块是不同单元。NSH 直接定义可执行模块，但宿主仍需开放足够的业务 API |
| 更新兼容与治理 | 检查包、运行时要求、撤销和回滚；能力与预算按 Surface 配置 | Flutter 核心[没有内置 code push](https://docs.flutter.dev/resources/faq#does-flutter-support-code-push)；第三方补丁依赖对应发布版本，原生依赖需已存在 | NSH 将模块接受与执行策略放在一起；Flutter 宿主要根据 RFW、补丁或脚本路线另外设计模块级信任和限额 |
| Rust 计算与原生组件 | Rust 组件/模块直接进入 NSH 核心及其绑定接口 | [dart:ffi](https://docs.flutter.dev/platform-integration/bind-native-code)可直接调用 Rust 导出的 C ABI；原生视图另走平台视图集成 | 调用 Rust 两边都可行，也不必经过序列化消息桥。FFI 解决计算接口，不自动解决图表的 Widget、渲染和销毁适配 |
| Web 集成 | HTML/DOM 呈现；TSX 运行逻辑与纯 Rust 路径分开选择 | [Flutter Web 多视图](https://docs.flutter.dev/platform-integration/web/embedding-flutter-web)可嵌入宿主 DOM 元素，内部仍使用 Flutter 呈现体系 | 都能放进已有网页；区别在界面内部使用 DOM 还是 Flutter 的绘制/组件模型，而不是能否嵌入 |

**同样生成订单确认界面：**RFW 适合组合已提供的 Widget 和事件；要加入新的校验、查询与流程逻辑，需要确认逻辑能否由已开放的回调表达，或引入脚本/代码补丁。NSH 的 TSX 包直接承载这些逻辑，因此更贴合每次任务交付不同程序化界面的场景。

**同样复用 Rust 图表：**Flutter 可以通过 FFI 使用计算核心，再用 Flutter 绘图或平台视图展示；NSH 可以让 Rust 组件承担绘制和业务，TSX 负责组合。差异是组件和生命周期边界在哪里，不是只有 NSH 能执行原生 Rust。选择 NSH 也意味着需要核对现有 Flutter 控件、动画和插件的替代或适配工作。

### 与 GPUI 相比：直接编译的 Rust 视图，还是宿主中的动态 Surface

[GPUI](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md) 用 Rust 的 `Entity` 保存状态，以 `Render` 构造元素树，再由 GPU 绘制。NSH 的 [GPUI 后端](./gpui.md)目前基于 GPUI Fast：外围仍是 GPUI 应用，里面放一个拥有自身视图树的 NSH Surface。比较的关键是这层运行时带来了什么，也增加了什么。

| 维度 | NSH 的实际机制 | 直接使用 GPUI | 实际差异与代价 |
| --- | --- | --- | --- |
| 新交互如何到达用户 | 加载 TSX bundle，使用已编译进宿主的组件与服务 | Rust 视图和事件处理随应用编译；额外动态层由应用选择和实现 | NSH 可以在发布后交付新的任务逻辑；直接 GPUI 保留更直接的 Rust 类型和调用关系 |
| 状态与更新 | Rust 信号图追踪绑定；组件初始化后按依赖更新，核心维护影子树并用 Taffy 布局 | `Entity` 管理状态，视图通过 `Render` 生成元素并交给 GPUI 布局和绘制 | NSH 的响应式边界在 Surface 内；它不会自动把宿主的 `Entity` 变成模块状态，需要明确同步接口 |
| 调度与绘制 | 桌面 Surface 的引擎线程处理脚本、文本和布局，再向 GPUI 主线程提交批次；一个 Surface 元素绘制内部保留树 | UI 操作遵守 GPUI 上下文与前台执行规则，后台工作使用其执行器 | NSH 把模块逻辑与挂载分开，但引入自己的布局、事件和数据转换路径；不能据此推断整体更快 |
| 组件与生命周期 | `AppView` 管理 Surface；嵌入点和暴露能力由宿主确定 | GPUI 元素、视图和 Rust 对象在同一应用模型内直接组合 | 包住一个动态区域很直接；把任意已有 GPUI 组件放进动态树仍需实际适配，不是导入 crate 后自动可用 |
| 模块治理 | 签名验证、撤销检查、能力策略及 VM 预算按 NSH 路径配置 | GPUI 的视图和执行器是应用基础设施；独立包信任、脚本运行时及权限策略由应用另行设计 | 需要任务扩展时，NSH 提供现成的管理单元；完全受信任的 Rust 核心无需为此增加脚本运行时 |
| 平台边界 | 同一模块模型还可通过 NSH 移动后端与 Web 路径承载 | 上游提供桌面后端及[浏览器示例](https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/README.md) | NSH 的移动接入是另一套后端能力，不等于把既有 GPUI 桌面交互直接移植到手机 |

**放到同一任务里看：**已有 GPUI Kit 应用要展示新生成的订单确认面板，NSH 可以让外围导航和数据服务保持 Rust 实现，只加载面板逻辑。若面板完全可预先实现，直接用 GPUI 组件通常减少一层状态、布局和事件边界。对于 Rust 图表，两边都可以保留编译后的实现；NSH 的额外价值是让运行时模块调用它，而不是让 Rust 本身更快。

调度依据：[GPUI 执行器](https://github.com/zed-industries/zed/blob/main/crates/gpui/src/executor.rs)、[NSH 架构](./architecture.md)与 [GPUI 后端](./gpui.md)。

### 与 Qt 相比：动态 QML 已存在，信任模型与宿主边界才是重点

Qt Quick 与 Qt Widgets 需要分开看。Quick 的 QML 本来就有动态对象、属性绑定与 JavaScript 行为；Widgets 是 QWidget 对象与布局工具箱，也能加载 UI 文件。NSH 更有价值的差异，是怎样接收不同来源的模块，以及这些模块怎样调用宿主。

| 维度 | NSH 的实际机制 | Qt Quick / Qt Widgets 的实际机制 | 实际差异与代价 |
| --- | --- | --- | --- |
| 动态表达 | TSX bundle 带状态、异步逻辑和事件，通过 Surface 加载 | Quick 可[加载 QML 文件或动态编译字符串](https://doc.qt.io/qt-6/qtqml-javascript-dynamicobjectcreation.html)，包含 JS 处理函数；Widgets 的 [QUiLoader](https://doc.qt.io/qt-6/quiloader.html)创建已有类型的控件 | Quick 同样能生成新交互；Widgets 的 UI 描述与新增程序逻辑是两层。NSH 的区别在模块执行契约，不是唯一支持动态 UI |
| 状态与布局 | Rust 信号图、Taffy flex/grid 和挂载批次 | QML 的[属性绑定](https://doc.qt.io/qt-6/qtqml-syntax-propertybinding.html)按依赖更新，使用 anchors/Layouts；Widgets 使用 [QLayout](https://doc.qt.io/qt-6/layout.html)管理控件尺寸 | QML 与 NSH 都有细粒度依赖更新；组件、布局规则和业务对象模型不同，不能仅凭“响应式”断言一方性能更好 |
| 执行与渲染 | 原生 Surface 将脚本和布局放在引擎线程，平台后端负责呈现 | QML 引擎执行绑定/处理函数，可使用[编译优化](https://doc.qt.io/qt-6/qtqml-qtquick-compiler-tech.html)；Quick 场景图有[单线程或线程化渲染循环](https://doc.qt.io/qt-6/qtquick-visualcanvas-scenegraph.html)，Widgets UI 在主线程 | NSH 的分隔点是 Surface 提交；Quick 的分隔点是 GUI 对象与场景图。两者原生扩展都必须遵守线程要求 |
| 信任与预算 | 包验证后仍执行能力检查、VM 堆/栈/调用时间及节点限制 | Qt 的[安全模型](https://doc.qt.io/qt-6/shared-security-model.html)假定受信任应用代码；不支持把不可信 QML 当成隔离域。Qt 也有 [QJSEngine 中断接口](https://doc.qt.io/qt-6/qjsengine.html) | Qt 可构建受控脚本宿主，但独立模块授权、限额和隔离策略需要额外设计；NSH 已把一部分策略纳入模块接口，仍不是进程级隔离 |
| 宿主对象与销毁 | 宿主注册组件工厂、属性、事件和命令，Surface 管理动态树；业务资源另有授权与取消规则 | QML 使用 QObject 的属性/信号/方法，需遵守 [C++/QML 所有权规则](https://doc.qt.io/qt-6/qtqml-cppintegration-data.html)；Loader 可销毁子树 | 两者都可卸载界面，但都不能把“删掉 UI”当作撤销已发出的业务操作；对象和异步任务的所有权必须明确 |
| Rust 与已有 UI | Rust 组件和业务类型可直接接入 NSH；已有平台视图走注册接口 | [CXX-Qt](https://kdab.github.io/cxx-qt/book/concepts/generated_qobject.html)等桥接 QObject 与 Rust；[QQuickWidget](https://doc.qt.io/qt-6/qquickwidget.html)把 Quick 放入 Widgets，但增加离屏渲染步骤并影响渲染线程模式 | Qt 可以复用 Rust，也有具体的嵌入代价。NSH 更贴合现有 Rust/原生宿主；Qt 产品则保留成熟 QObject、控件与工具积累 |
| UI 模块与二进制插件 | 动态包使用宿主已有能力，新增本机实现随宿主交付 | QML 源码加载与 [QPluginLoader](https://doc.qt.io/qt-6/qpluginloader.html)共享库插件是不同路径，后者有平台/ABI 部署要求 | 两者都须区分界面逻辑与本机能力。NSH 签名包不是任意跨平台原生插件格式 |
| 浏览器呈现 | Web 后端输出 HTML/DOM | [Qt for WebAssembly](https://doc.qt.io/qt-6/wasm.html)支持 Widgets、QML 与 Quick，以浏览器绘制路径承载并有平台限制 | 都能进入浏览器；NSH 的差异是 DOM 集成方式，不是独有 Web 支持，DOM 本身也不自动保证全部无障碍行为 |

**同样生成订单确认界面：**Qt Quick 可以直接加载带 JS 的 QML，并调用已开放的 QObject 方法，这一表达能力与 NSH 的目标有重叠。若代码全部由同一受信任团队交付，Qt 的动态组件和业务模型已经很完整。若接受来自多个合作方或模型生成的代码，Qt 官方信任假设就成为关键：宿主需要额外设计接受策略和执行边界，NSH 则将其中的包验证、能力交集和 VM 预算作为现有接口提供。

**同样撤销模块：**Qt Loader 可以移除界面，QJSEngine 可以中断执行；NSH 提供包撤销检查与按 Surface 配置的限制。两者仍需宿主处理进行中的原生调用和业务副作用。NSH 的价值在于将这些接受与执行策略串成模块流程。

### 与 Slint 相比：同为 Rust 友好，交付语言与模块治理不同

Slint 是 NSH 很有意义的对照：它也连接 Rust、支持细粒度绑定，并有运行时解释器。两者的取舍在于新交付的程序是什么、宿主怎样接入，以及生命周期和策略由哪一层管理。

| 维度 | NSH 的实际机制 | Slint 的实际机制 | 实际差异与代价 |
| --- | --- | --- | --- |
| 交付的界面程序 | TSX 包组织状态、Promise/异步操作与事件，运行在 QuickJS | 通常将 `.slint` 编译成组件类型；[解释器](https://docs.slint.dev/latest/docs/rust/slint_interpreter/)也能从源码/文件构建组件，设置属性并接入回调 | 两者都能运行时生成交互；NSH 交付通用 TSX 模块，Slint 使用自己的 UI 语言并通过宿主回调连接业务 |
| 状态求值与布局 | 信号图追踪依赖，更新绑定后由 Taffy 布局 | Slint 绑定按依赖标脏，在读取时[惰性重新求值](https://docs.slint.dev/latest/docs/slint/reference/language/evaluation-and-purity/)；布局使用最小/最大/首选尺寸与伸缩约束 | 同为细粒度模型，求值与布局规则不同。选择应看组件和任务逻辑是否自然表达，不是简单对比“是否重渲染” |
| 调度 | 原生动态 Surface 使用独立引擎线程，UI 线程应用批次 | 组件通常在 UI 事件循环线程使用；后台结果通过 [invoke_from_event_loop](https://docs.slint.dev/latest/docs/rust/slint/)排回 UI | NSH 为脚本模块提供独立调度单元；Slint 的宿主需要安排耗时工作，NSH 的原生回调同样要自行管理阻塞 |
| 对象与回调生命周期 | Surface 管理模块和节点，宿主注册对象保留自己的资源责任 | [ComponentHandle](https://docs.slint.dev/latest/docs/rust/slint/trait.ComponentHandle)是强引用；回调捕获通常使用 [Weak](https://docs.slint.dev/latest/docs/rust/slint/struct.Weak)，跨线程更新进入事件循环 | Slint 的生命周期规则是显式且可控的；NSH 将模块生命周期集中到 Surface，但异步回调和宿主资源仍需协作清理 |
| 包接受与权限 | 签名、撤销、回滚、能力交集和 VM 限额有对应接口 | 解释器提供编译、导入和组件 API；导入钩子可以限制来源，独立包信任/权限/预算由宿主实现 | NSH 减少为合作方模块搭建策略层的工作；这不是说 Slint 无法实现，而是框架负责的范围不同 |
| Rust 计算与绘制 | Rust 核心组件可供 TSX 使用，宿主平台视图可按注册接口接入 | Rust 属性/回调 API 连接业务；可将 Rust 生成的像素经 [SharedPixelBuffer/Image](https://docs.slint.dev/latest/docs/rust/slint/struct.Image)显示，或接入渲染器 | Rust 计算不是 NSH 独有优势；已有 Slint 组件可直接保留，NSH 则让动态任务模块与 Rust 能力共享其组件模型 |
| 新的原生能力 | 更新宿主二进制，再允许模块调用 | 动态解释 `.slint` 也不等于加载新的 Rust 本机实现 | 两者都受宿主 API 和二进制兼容约束；NSH 的动态包不消除原生发布流程 |
| 移动与 Web | iOS/Android 后端与 HTML/DOM Web 路径 | Slint 有 [Android](https://docs.slint.dev/latest/docs/slint/guide/platforms/mobile/android/)和 [iOS](https://docs.slint.dev/latest/docs/slint/guide/platforms/mobile/ios/)路径，[Web](https://docs.slint.dev/latest/docs/slint/guide/platforms/web/)通过 Canvas 绘制 | 区别在平台集成与呈现模型。已有 Slint 产品迁移时要比较具体输入、渲染和嵌入需求，不能假设它没有移动端或浏览器支持 |

**同样生成订单确认界面：**Slint 解释器可以构造新的界面和处理逻辑，再接上宿主保存订单回调；NSH 可在 TSX 中组织异步查询和事件流程，再调用注册 API。需要复杂通用任务逻辑时，NSH 的语言和模块执行路径更直接；如果已有 Slint 组件与回调足够表达需求，继续使用它可避免引入另一套运行时。

**同样复用 Rust 图表：**Slint 和 NSH 都可保持 Rust 计算与绘制实现。NSH 的收益在于已有 Surface 包和宿主能力契约；Slint 的收益在于现有原生组件/API 和可选择的编译路径。双方都需要把绘制资源、线程和销毁责任接好，不能从语言一致推导“零适配”。

## 动态能力与编译后的核心共同存在

动态界面需要灵活性，核心功能需要可预测的实现方式。NSH 允许应用分别选择它们的执行路径。

| 功能层次 | 可采用的实现 | 交付与执行方式 |
| --- | --- | --- |
| 核心业务与计算 | Rust 模块、类型与后台任务 | 在原生平台编译为本机代码，由宿主开放调用接口 |
| 复杂或高频 UI | Rust 组件或宿主已有原生组件 | 核心实现留在原生侧，动态模块组织交互 |
| AI 生成页面 | 动态 TSX 模块 | 按任务交付状态、布局与行为 |
| 完整应用 | 纯 Rust、TSX 或混合方式 | 按产品需要决定动态能力的范围 |

[纯 Rust 应用](./rust.md)可以完全不链接 QuickJS；混合应用为动态 TSX 保留它。图表、编辑器和数据处理等部分可以继续使用 Rust，让动态模块承担页面组合和任务交互。

在原生平台上，Rust UI 和业务编译为本机代码，其执行不依赖 JavaScript 引擎的处理能力，可以直接使用 Rust 的内存管理与底层优化手段。动态模块因而可以专注于页面组合，核心实现按自身的性能需求演进。最终界面表现仍需结合布局、文本、绘制和平台适配，在目标产品与设备上实际测量。

## 平台能力的选择

| 平台 | NSH 的界面路径 | 集成方式 |
| --- | --- | --- |
| iOS | UIKit 与 CoreText | 嵌入原生 App 或构建独立应用 |
| Android | Android Views 与平台文本、输入集成 | 嵌入已有客户端或构建独立应用 |
| macOS | AppKit、Core Animation 与 CoreText | 嵌入或构建桌面客户端 |
| Windows | Win32 中的保留视图树，Direct2D 与 DirectWrite 绘制 | 嵌入或构建桌面客户端 |
| Linux | GTK 4、自定义 GSK 绘制与 Pango 文本 | 嵌入或构建桌面客户端 |
| GPUI | 通过 GPUI 后端绘制 NSH 视图树 | 嵌入 GPUI 和 GPUI Kit 产品 |
| Web | HTML 与 DOM | 发布网页界面或接入已有页面 |

NSH 共享组件抽象与开发方式，同时采用适合各端的实现。共享组件不意味着每个控件都由操作系统控件提供，也不意味着各端行为完全相同；请按实际需求核对[平台指南](./platforms.md)。

在 [Web](./web.md) 上，Rust 和 TSX 应用都呈现为 HTML。动态 TSX 使用 QuickJS，纯 Rust 应用可以移除这一依赖。浏览器中的呈现方式与应用逻辑的执行语言是两个不同维度。

## 哪些产品适合选择 NSH

| 产品需求 | NSH 的使用方式 | 选择时重点确认 |
| --- | --- | --- |
| AI 助手生成可操作结果 | 交付表单、筛选器、列表与步骤操作界面 | 生成代码的检查与宿主业务授权 |
| 手机客户端按场景交付页面 | 动态模块调用宿主开放的服务 | 平台政策、模块信任与原生能力范围 |
| 桌面插件和工作流扩展 | 用签名包和 Surface 策略组织模块 | 更新兼容、生命周期与资源预算 |
| 已有原生产品逐步增加 AI 能力 | 从一个局部面板或页面开始 | 嵌入接口与既有页面的协作 |
| GPUI 产品需要动态界面 | 保留 Rust 主体，嵌入 NSH Surface | 宿主能力注册与实际平台集成 |
| 完整 Rust 客户端 | 用 Rust 构建 UI 和业务，按需加入动态模块 | 需要的组件、平台行为与性能实测 |
| 手机、桌面和 Web 的交互入口 | 使用共享组件模型与各端后端 | 各端输入、导航、呈现和集成差异 |

NSH 的核心价值，是让应用在运行时获得新的交互能力，同时保留宿主对功能、权限和实现方式的控制。可以从一个 AI 生成的动态面板开始，也可以用同一套能力构建完整客户端。选择框架时，将这条交付路径与现有团队、组件积累、平台需求一起评估，才能判断它能为产品带来什么。
