---
title: Comparing approaches
description: Compare Native Shell with React Native, Flutter, GPUI, Qt and Slint, from AI-generated interfaces and dynamic delivery to host composition and complete Rust applications.
order: 2
group: start
---

# Choosing technology for interfaces that arrive at runtime

Native Shell (NSH) lets mobile and desktop applications receive new interactive interfaces after release. It brings programmable interfaces, dynamic module loading, host component composition and execution management into one development path: the host retains control of the stable product core, while new task interfaces can arrive on demand.

This matters especially for AI products. Only after a user asks for something does the application know which forms, lists, charts and interaction flows it needs. NSH turns the result into an interface with state and behavior, using business services and native capabilities already available in the client.

**The main reason to choose NSH is that your product needs a continuing supply of new interactions while preserving the existing client and its control over execution.** NSH also supports complete applications built in Rust, with dynamic TSX added where useful. These paths share components and the core engine. Dynamic delivery can serve one panel or become the product's main delivery model.

## Start with a client that has already shipped

Suppose your product already has order services, login state, charts, and mobile and desktop clients. A user now asks for a task that was never designed before release:

“Compare these quotes, fill in the missing information, choose a delivery method, and save the order after I confirm.”

This requires more than placing text in a chat bubble. The interface must query data, retain selections, handle failures, change the next step based on input, and call the existing save-order API. The next task may need a different flow again.

Three approaches are reasonable, with engineering costs in different places:

| What you actually need to deliver | A suitable starting point | Engineering work as requirements grow |
| --- | --- | --- |
| New data, a new layout or a new combination of known actions on an existing page | Server-driven UI, or corresponding technologies such as Flutter RFW | Maintain the component and action contract; extend the host or expression language when new behavior exceeds it |
| A new version of your own application's code | React Native update services, an application release or a suitable code-patching solution | Manage application versions, native dependency compatibility, testing and rollback |
| An independently loaded task module with its own state and logic | NSH, or a corresponding module host built on another framework | Define exposed APIs, module sources and permissions; NSH provides the foundation for package verification and surface execution management |

A WebView is also a direct and effective choice, especially for existing pages, documents and web components. Evaluate the cost of integrating that content with native services, input and views. NSH serves products that want their client backends to present these dynamic interfaces while reusing host components.

## How NSH turns this task into an interface

**Expose capabilities first, then deliver the flow.** The host provides interfaces such as “read quotes,” “save order” and charts. The dynamic module decides how to compose them, managing current selections, query progress, validation and confirmation. Order permissions, login credentials and final business validation stay in the host or on the server. The module works through those interfaces rather than receiving the application's entire internal object model.

A deliverable version can reach the client through this sequence:

1. Write or generate TSX with typed components and APIs, including the task's layout, state and event logic. Developers can use the [development tools](./cli.md) to check interfaces and [headless traces](./testing-and-trace.md) to inspect execution.
2. Build the module as a bundle; use a [signed package](./signed-packages.md) for dynamic delivery that needs provenance and version checks. The host chooses download sources and update timing. NSH performs trust, compatible-version, revocation and rollback checks when opening a signed package.
3. Create a surface in the page and configure its allowed capabilities and execution budget. Each native dynamic TSX surface has its own QuickJS VM and engine thread, with interface updates handled by the corresponding platform backend.
4. Unload the surface when the task ends. The host cancels associated requests, cleans up business resources, and decides when a running module needs to stop or reload.

A chart can remain a Rust component, or an existing host view can be integrated through [native component registration](./native-components.md). The dynamic module handles composition; the host supplies properties, events, commands and sizing interfaces. New native binary capabilities still require a host update. Modules call capabilities that already exist and have been exposed to them.

For the product, this means **new flows can ship independently, core business logic and complex components remain reusable, and the host decides each module's permissions.** When comparing other frameworks, consider what this path lets you retain, what you need to add and whether that work is worthwhile.

## React Native: keep updating React products; extend native and Rust hosts with modules

NSH lets existing native and Rust clients build extension interfaces directly around deliverable modules, keeping core components and business implementations in the host.

If the order client already uses React Native, the new panel can be a React component delivered through [EAS Update](https://docs.expo.dev/eas-update/runtime-versions/) as compatible JavaScript and assets, still calling the installed order module. This preserves the React state model, component libraries and native extensions. AI generation alone is not a reason to switch to NSH.

NSH becomes more attractive when the product already has native or Rust clients and needs to add task interfaces loaded on demand. For example, the mobile client keeps its native order page and the desktop client keeps its Rust chart, while only the “compare quotes and confirm” region is dynamically delivered. The host exposes capabilities through NSH's component registration and Rust binding APIs, and modules organize interactions around those interfaces.

React Native also supports [embedding a view or flow in a native application](https://reactnative.dev/docs/integration-with-existing-apps). The practical integration choices are:

- With React Native, connect existing views to [Fabric](https://reactnative.dev/docs/fabric-native-components-introduction), expose business capabilities through Native Modules, and maintain the dynamic region around React's state and component model. The New Architecture uses JSI to work with native implementations.
- With NSH, register host views as components, expose Rust components and business types to TSX, and manage the region's execution as a surface. NSH's signal-based component model requires adaptation for React Hooks and existing React components.

**Your existing component assets therefore tell you more about integration cost than the fact that both use TSX.** React Native teams can often reuse existing pages directly. Rust and platform-native teams can build NSH interfaces around their existing implementations, keeping those implementations in their current form.

When a product installs modules from several independent sources, the delivery unit also matters. EAS handles JavaScript and asset updates for a particular native runtime and provides [signature verification](https://docs.expo.dev/eas-update/code-signing/). NSH connects package acceptance policies with surface capabilities and VM budgets to establish boundaries for each module. React Native's [ReactHost](https://github.com/react/react-native/blob/main/packages/react-native/ReactAndroid/src/main/java/com/facebook/react/ReactHost.kt) can manage multiple surfaces, but a surface by itself does not imply an independent JavaScript VM. Runtime allocation and module policies still need to be explicit in the design.

React Native's embedding capabilities are evolving too: [Expo SDK 56](https://expo.dev/changelog/sdk-56) provides an experimental path for embedding multiple applications on iOS. It is an existing solution worth evaluating against target platforms, versions and module permission requirements, with the specific guarantees for multiple instances and permission isolation checked separately.

**How to choose:** when you mainly deliver your own trusted React pages, start by extending React Native. For an existing native or Rust product that needs loadable interaction modules as a long-term extension mechanism, NSH's interfaces and execution unit map more directly to that goal. The benefit is less module-host infrastructure to build; React pages still require migration work.

## Flutter: decide whether to deliver widget descriptions, application patches or a new program

NSH can deliver new asynchronous interaction flows with a module, letting the host expose stable business APIs while individual task flows arrive on demand.

Flutter's appeal is the ability to build a complete experience around one widget, animation and rendering system. If the quote flow is a fixed core application feature, implementing it in Dart and reusing existing widgets is usually the most direct route. NSH's advantage emerges when the flow keeps changing after release and those changes include new program logic.

Consider a flow that must “re-query an expired quote, retain selections that are still valid, then ask the user to confirm the price difference.” Three dynamic approaches lead to different work:

| Approach | What is delivered this time | Where the new logic lives |
| --- | --- | --- |
| Flutter [Remote Flutter Widgets (RFW)](https://pub.dev/packages/rfw) | Remote widget descriptions, data and event connections | Use expressions supported by the description language and events already provided by the host; queries and business behavior need corresponding host support, with scripting as an additional option |
| Flutter application patches, such as [Shorebird](https://docs.shorebird.dev/code-push/) | Dart code patches targeting an existing release | Update the application program within patch and native-code compatibility boundaries |
| NSH dynamic modules | TSX with state, asynchronous queries, validation and event handlers | The module organizes the new flow and calls services already exposed by the host |

If the host already provides the complete flow, RFW can recombine it. If every generated task introduces new branches, implementing every possible behavior as a host callback makes the action protocol keep growing. NSH lets that orchestration travel with the module: the host maintains stable business APIs while modules define individual task flows. Application patches can also change program behavior, but their unit is the evolving application version. Independently installed modules still need their own installation, unloading and authorization contracts.

The second choice is **who draws the interface and how existing UI is reused**. Flutter usually draws widgets through its own engine; embedding existing native controls requires [platform-view integration](https://docs.flutter.dev/platform-integration/android/platform-views). NSH uses platform backends and provides registration APIs for host views. A product with an existing native map, player or input region should test focus, gestures, layout and lifecycle directly. Similar screenshots say little about these integration details. For a map or camera preview, for example, Flutter's Android platform views offer different composition paths, each with scrolling, accessibility and device conditions to evaluate. Registered NSH mobile views enter the host view hierarchy, but properties and events cross the boundary as data, and events are asynchronous. Gestures that need an immediate decision should stay on the native side. Existing Flutter widgets also need adaptation to be reused with NSH.

Flutter itself supports [add-to-app and multiple engines](https://docs.flutter.dev/add-to-app/multiple-flutters), and can call Rust through [dart:ffi](https://docs.flutter.dev/platform-integration/bind-native-code). The choice of NSH should follow the required delivery unit and component boundaries. Both frameworks support embedding, and Rust support alone establishes no performance ranking.

**How to choose:** Flutter fits products built around a unified widget, animation and plugin system. NSH is more direct when an existing client must continually receive new task programs built from its native and Rust components. Both support complete applications; the difference is which layer owns the part of the product that changes most often.

## GPUI: give a Rust client another way to deliver features

For an existing GPUI or GPUI Kit product, NSH's most practical role is a dynamic extension layer.

With [GPUI](https://github.com/zed-industries/zed/blob/main/crates/gpui/README.md) directly, the order panel's state, elements and event handlers are part of the Rust application. The compiler checks call relationships, and components can use host entities and services directly. This suits editors, complex desktop tools and stable core workflows.

A user-requested flow that was not implemented before release needs another expression mechanism: a broader data-driven protocol, a script runtime or a new application version. NSH provides one complete path: put a surface inside the GPUI application, let a new TSX module describe the flow, and call Rust capabilities already exposed by the host.

For example, navigation, the order model and frequently updated charts can remain in GPUI while NSH handles a user-generated comparison panel. Similarly, an editor's document model, selection and undo stack belong naturally in GPUI, while the parameters and step-by-step confirmation for one refactoring can become a dynamic task region. The concrete integration work is checking whether a document version read by the module remains valid, whether asynchronous results can still be applied, and which tasks to cancel when the panel closes. This lets the surrounding client retain its implementation. The current [NSH GPUI backend](./gpui.md) uses GPUI Fast and maintains its own view tree, so the GPUI host's state and the dynamic tree need explicit interfaces. Existing GPUI components also need their integration requirements checked before they can enter an NSH tree.

**How to choose:** when the core interface is known at compile time, direct GPUI is simplest. When the core is Rust but the product needs new user tools, partner modules or AI interactions after release, combine GPUI and NSH. This adds a runtime boundary in exchange for independently delivered interface logic. The rest of the product can stay in Rust, and mobile support for existing GPUI interfaces remains a separate adaptation task.

## Qt: dynamic QML is already comprehensive; the question is what kind of host you need

NSH brings package acceptance, host authorization and execution budgets for interfaces from multiple sources into one module path, making it a fit for products that treat dynamic extensions as a long-term capability.

Qt Quick is one of the strongest alternatives to examine closely. It can [load QML dynamically](https://doc.qt.io/qt-6/qtqml-javascript-dynamicobjectcreation.html), handle interaction with property bindings and JavaScript, and call exposed QObject methods. The order comparison panel can be built this way. Existing QML components and QObject business interfaces are valuable assets.

The distinction is the boundary around which the product is organized.

**Qt Quick starts naturally from a world of Qt objects.** New QML creates objects, binds properties, connects signals and reuses types already exposed by the application. For a desktop workbench, those assets also include [Model/View](https://doc.qt.io/qt-6/model-view-programming.html) data models, shared selection, sorting proxies and editing delegates. Replacing the interface layer means reconnecting those relationships too. This model suits Qt teams delivering new interface components and managing an application within a common toolkit. When the main challenge is developing and maintaining your own cross-platform interface more efficiently, Qt's controls, tools and platform modules deserve early consideration.

**NSH starts naturally from a host that receives modules.** Interfaces can come from several publishers. The host chooses which packages to accept, which capabilities to expose to a surface and how much execution resource to allocate. Package verification, runtime requirements and execution budgets have corresponding APIs. Qt's documented [security model](https://doc.qt.io/qt-6/shared-security-model.html) assumes trusted application code. Turning downloadable QML into a multi-source extension platform requires additional design for which code to trust and how to bound its execution. NSH includes these requirements in its product path, while its in-process VM still provides a different boundary from an operating-system sandbox.

A concrete question is whether you are willing to give a partner's new quote panel the capabilities exposed in the current QML environment. If so, and all code passes through one trusted release process, Qt's dynamic components may already meet the need. If each module needs a narrower network scope, script resource limits and package version checks, NSH's existing management APIs become more useful. Both still need protected business APIs, and unloading an interface does not undo an order that has already been submitted.

Qt Widgets offers a different starting point. [QUiLoader](https://doc.qt.io/qt-6/quiloader.html) creates existing control types from UI descriptions, while new business behavior needs corresponding code. A Widgets product can add a dynamic Quick region through [QQuickWidget](https://doc.qt.io/qt-6/qquickwidget.html), with an extra offscreen rendering step and render-loop constraints. These are concrete integration costs to evaluate alongside dynamic loading support.

Rust support is shared ground too: Qt can connect to Rust through solutions such as [CXX-Qt](https://kdab.github.io/cxx-qt/book/). **For an existing Qt product, first weigh the benefit of retaining QObject and QML assets. For an existing native or Rust host that needs dynamic module delivery and execution management as product capabilities, evaluate NSH closely.**

## Slint: a clear UI/host split, or task modules that orchestrate their own flows?

NSH lets deliverable TSX carry both the interface and task orchestration, fitting products whose new requirements often involve queries, waiting, retries and step-by-step confirmation.

Slint works closely with Rust, supporting both compiled UI and an [interpreter](https://docs.slint.dev/latest/docs/rust/slint_interpreter/) that loads `.slint` at runtime. Its language supports bindings, conditions, expressions and callback logic for dynamic interactions.

The useful comparison is where new business flows live.

If the quote panel displays data, switches selections and invokes the host's existing “submit order” callback, a dynamic Slint component can handle the task well. Rust owns services, tasks and business state; UI cooperates through properties and callbacks. The boundary is clear, and existing Slint components remain easy to reuse.

Now take another step: a new module queries several sources in parallel, handles partial failures, waits for user corrections, then chooses which host operation to call. With Slint, the design must divide that orchestration between the UI language and Rust callbacks. Loading new UI through the interpreter uses the Rust implementations already available in the host. With NSH, the TSX module can organize the flow through general-purpose scripting and asynchronous logic, while Rust exposes stable query and save APIs.

**The difference is how much host behavior each new task needs to extend.** When changes are mainly in the UI and the callback contract stays stable, Slint's division of responsibilities is natural. When changes frequently include new task orchestration, NSH can keep more of that change inside the deliverable module. The trade-off is a script runtime for the dynamic part and the need to manage generated code; the stable core can remain in Rust.

Both can use Rust for capabilities such as charts. Slint can display Rust-generated pixels through [Image / SharedPixelBuffer](https://docs.slint.dev/latest/docs/rust/slint/struct.Image) and connect data through properties and callbacks. NSH can expose Rust components or register host views. Evaluate whether the asset to reuse is a computation function, rendered output or an interactive component: each calls for different integration work.

**How to choose:** Slint is attractive when a Rust product needs a clear division between UI and business logic, especially with existing Slint components. When the goal is to deliver task modules with their own orchestration logic and compose host capabilities into new interactions, NSH's module language and execution path fit more directly.

## Complete applications can start in Rust too

NSH's dynamic capabilities can sit alongside a Rust application core. When [building a complete application in Rust](./rust.md), the UI, state and business logic can all be implemented in Rust, with QuickJS omitted entirely. On native platforms, those implementations compile to machine code and can use Rust's type checking, memory management and low-level optimization capabilities.

A mixed application takes a different division of responsibility: charts, editors, data processing and stable business logic remain compiled, while TSX handles frequently changing pages and task flows. Both paths share NSH's component and reactive core and compose through explicit interfaces. Building a complete Rust application first, then adding dynamic delivery to one region later, is one possible evolution path.

This also defines how to assess performance: measure actual layout, text, drawing, cross-boundary calls and task workloads. The native Rust core runs independently of JavaScript execution throughput. Whole-application performance relative to other frameworks still requires measurement beyond the implementation language.

## Choose platforms and execution boundaries together

Platform backends implement NSH's component abstractions: iOS uses UIKit/CoreText; Android uses Android Views and platform input capabilities; macOS uses AppKit/Core Animation/CoreText; Windows uses Win32/Direct2D/DirectWrite; and Linux uses GTK 4/GSK/Pango. The GPUI backend serves Rust desktop hosts, and Web presents HTML/DOM. See the [platform guides](./platforms.md) for integration details and supported scope. Shared components use a mixture of system controls and backend implementations, and behavior varies by platform.

The [Web path](./web.md) needs separate consideration. Dynamic TSX still uses QuickJS, while pure Rust can omit it; both present HTML through the Web backend. The browser-local engine runs on the page thread, so native surface threading arrangements do not carry over directly to Web.

When accepting generated code, establish the following boundaries, detailed in [capabilities and resource limits](./sandbox-and-capabilities.md):

- Signatures establish provenance and integrity, not behavioral safety. QuickJS bytecode needs a trusted source, such as a verified package. An in-process VM does not provide operating-system process isolation.
- Configure capability policies explicitly. Omitting a module capability declaration does not deny all access; host grants and module declarations jointly determine the effective scope of managed capabilities.
- VM heap, stack, call-duration and node limits do not bound the full CPU, memory or GPU cost of arbitrary native host functions. Business authorization, expensive operations and cancellation remain the host's responsibility.
- The host fetches revocation information and decides how to handle running modules. Revocation checks when a package opens do not remotely terminate it immediately, and removing UI does not roll back business side effects.

Dynamic modules must also follow the target platform's distribution policies. These execution mechanisms operate within native release and platform review requirements.

## Finally, choose around the core you want to keep

| Product starting point and goal | Path to evaluate first |
| --- | --- |
| AI-generated interfaces with new queries, validation and step-by-step operations | NSH delivers the interface and task logic as one module |
| Partner extensions that need independent versions, permissions and execution budgets | NSH signed packages, surfaces and host policy APIs |
| Existing native or Rust clients that need to reuse their core while continually adding new interfaces | NSH dynamic modules composed with host components |
| Complete UI and business logic in Rust, with dynamic behavior chosen region by region | NSH's pure Rust or Rust + TSX path |
| Existing React Native components and team, continually updating your own mobile pages | React Native and existing update services |
| Building a complete product around a unified widget, animation and plugin system | Flutter |
| A stable Rust desktop core that needs direct type and component collaboration | GPUI directly; combine with NSH when runtime extensions are needed |
| Existing QObject/QML/Widgets assets, primarily running trusted application code | Qt; evaluate execution boundaries separately for modules from multiple sources |
| Stable separation between Rust business logic and UI properties/callbacks, with existing Slint components | Slint, including its runtime interpreter |

NSH's clearest value is to turn interfaces created after release into units a client can accept, compose and manage. For products that need AI interactions, dynamic business pages or extension modules, this directly affects how new features reach users and which capabilities can remain in a stable native core over time.

This article is based on NSH's implementation and the linked primary sources as of October 19, 2026. Its engineering trade-offs are analysis of those mechanisms, rather than cross-framework performance benchmarks.
