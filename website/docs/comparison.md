---
title: Comparing approaches
description: How Native Shell combines AI-generated interfaces, runtime delivery and host capabilities, and how that direction compares with other cross-platform UI frameworks.
order: 2
group: start
---

# Comparing approaches

Native Shell (NSH) is a cross-platform UI engine for **AI-generated interfaces, dynamic delivery and composition inside a host application**. An app can load an interface that was not written when it shipped: a task-specific form, an interactive result, a partner module or a user-installed extension. The module carries state and interaction logic; the host chooses the components, services and resources it can use.

NSH also supports complete mobile and desktop applications. Use dynamic TSX modules, compiled Rust components and host-provided native views together, or build a [pure Rust application](./rust.md) without linking QuickJS. Dynamic delivery and a compiled core can share the same application.

## From a task to an interactive interface

An AI assistant may need a filterable list for one answer and a confirmation workflow for the next. NSH gives these interfaces a path from generation to execution:

- **Describe the interaction:** typed TSX APIs cover components, state, events and asynchronous work. [Headless checks and traces](./testing-and-trace.md) help inspect generated interfaces before integrating them.
- **Deliver the module:** load bundles or verify [signed packages](./signed-packages.md), including runtime compatibility and version checks. The host supplies the download service and decides when to accept an update.
- **Compose with the application:** embed a surface in an existing [iOS](./embedding-ios.md), [Android](./embedding-android.md) or [GPUI application](./gpui.md). Expose [native components](./native-components.md) and Rust business APIs through the host.
- **Set execution boundaries:** give each native dynamic surface its own VM, resource budgets and an explicit capability policy. Keep authorization for business operations in the host.

This is runtime delivery of interface code and behavior, beyond changing data in a prebuilt screen or using development hot reload. New native binary capabilities still require a host build and release. Distribution also remains subject to the target platform's policies.

## Other ways to deliver dynamic UI

| Approach | What arrives at runtime | What the host manages |
| --- | --- | --- |
| **WebView** | HTML, CSS and JavaScript, with the browser's layout and rendering | Web/native integration, navigation and access to native services |
| **Server-driven UI** | A schema of components and actions; expression and state support depend on the protocol | The component catalog, protocol compatibility and supported behavior |
| **Application code updates** | Code and assets compatible with the installed runtime | Update selection, native compatibility and the application's trust model |
| **NSH modules** | Programmable TSX interfaces composed with registered host capabilities | Module verification, surface lifecycle, execution budgets and capability policy |

These approaches can coexist. A WebView remains useful for web content; a schema can be sufficient for a predictable form. NSH focuses on modules that need their own interaction logic while using a host's existing native and Rust capabilities.

## Framework directions

The comparison is about product fit, not a performance ranking. These frameworks can be extended, and several support embedding and dynamic loading already.

| Framework | Main approach | Dynamic or embedded UI path |
| --- | --- | --- |
| **NSH** | Runtime-delivered interfaces and host composition, with Rust and TSX application paths | Surface loading, signed packages, host capabilities and execution limits |
| **React Native** | React-driven native applications and native extensions | Embed individual views or flows; deliver compatible JS and assets through update services |
| **Flutter** | A shared widget, animation and rendering system across platforms | Add-to-app, platform views, server-driven widgets and third-party code updates |
| **GPUI** | Rust code-first GPU UI, primarily used on desktop | The application or an added runtime supplies dynamic modules; upstream also has a browser path |
| **Qt Quick / Qt Widgets** | QML and a scene graph / a traditional desktop widget toolkit | Load and create QML components dynamically; construct widgets through application code |
| **Slint** | Declarative UI connected to Rust, C++ and other application logic | Compile UI or load it with the interpreter |
| **MyGo** | Go applications with native GPU UI or a system WebView frontend | Native Go UI, web content, and a WebAssembly browser target for native UI |

### React Native and Flutter

React Native supports [incremental integration into existing applications](https://reactnative.dev/docs/integration-with-existing-apps), including a single view or user flow. [EAS Update](https://docs.expo.dev/eas-update/introduction/) delivers non-native assets such as JavaScript and images to compatible runtimes; changing native capabilities requires a new binary. NSH puts the independently managed surface at the center of this workflow and offers a direct Rust implementation path for components and business logic.

Flutter's [architecture](https://docs.flutter.dev/resources/architectural-overview) provides a unified widget and rendering model, with add-to-app and platform views for native integration. Its [official FAQ](https://docs.flutter.dev/resources/faq#does-flutter-support-code-push) distinguishes third-party code push from built-in framework capabilities. NSH is a fit when the product centers on hosting runtime-delivered modules and combining them with existing host services. Existing React or Flutter teams should also weigh their component libraries, tooling and integration costs.

### GPUI

[GPUI](https://www.gpui.rs/) provides the Rust desktop UI foundation; GPUI Kit provides reusable components on top of that ecosystem. NSH can supply the dynamic interface layer in an existing desktop product. Its [GPUI backend](./gpui.md), currently built on GPUI Fast, embeds an NSH surface as a GPUI view. Keep the surrounding application and performance-sensitive components in Rust, and expose the capabilities a module needs. NSH supplies separate backends for mobile clients. Upstream GPUI also has [browser examples](https://github.com/zed-industries/zed/blob/main/crates/gpui/examples/README.md).

### Qt Quick and Qt Widgets

Qt Quick already supports [dynamic QML component creation](https://doc.qt.io/qt-6/qtqml-javascript-dynamicobjectcreation.html). [Qt Widgets](https://doc.qt.io/qt-6/qtwidgets-index.html) supplies a mature desktop toolkit. Both connect UI to native application logic. NSH's emphasis is the delivery and management of independently loaded modules: package provenance, host capabilities, resource limits and surface lifecycle. Teams that depend on Qt's established widgets and platform modules should include that investment in their choice.

### Slint and MyGo

[Slint's interpreter](https://docs.slint.dev/latest/docs/rust/slint_interpreter/) supports loading UI at runtime as well as the usual compiled UI path. NSH uses TSX modules to express state, asynchronous work and interaction, with package and host policies around their execution. Slint also supports state and imperative handlers; the distinction here is NSH's package and host-capability workflow, not whether an interface can contain logic.

[MyGo](https://mygo.egoist.dev/docs/architecture) offers Go developers both native UI and a system WebView path. Its native UI also has a WebAssembly browser target. NSH focuses on mobile and desktop module hosting and Rust integration. For a Go desktop product, MyGo's language and tooling may be a more direct fit; for a Rust or GPUI host adding dynamic task interfaces, NSH provides a related component and runtime model.

## Composition and performance

Start with a panel inside an existing app, add a dynamically delivered page, or build the entire client. A Rust chart, editor or business service can remain compiled while a TSX module organizes its controls and interactions. Hosts register the capabilities they intend to expose; modules do not introduce new native binaries by themselves.

[Pure Rust and mixed applications](./rust.md) use the same core component and reactive model. Pure Rust removes the QuickJS dependency; mixed applications retain it for dynamic TSX. This is an implementation choice, not a guarantee of faster UI: layout, text, rendering, business logic and platform integration still determine the result.

## Platform paths

| Target | NSH presentation path |
| --- | --- |
| iOS | UIKit and CoreText |
| Android | Android Views and platform text/input integration |
| macOS | AppKit, Core Animation and CoreText |
| Windows | A retained view tree drawn with Direct2D and DirectWrite in Win32 |
| Linux | GTK 4, custom GSK drawing and Pango text |
| GPUI | An NSH view tree drawn through the GPUI backend |
| Web | HTML and DOM presentation in the browser |

Shared components do not imply identical platform behavior or that every control is an operating-system widget. Check the [platform guides](./platforms.md) for the integrations your app needs.

On [Web](./web.md), both Rust and TSX applications render HTML. Dynamic TSX uses QuickJS; pure Rust applications can omit that dependency. Browser presentation and the choice of application language are separate decisions.

## Execution boundaries

[Signed packages](./signed-packages.md) verify origin and integrity through trust roots, release certificates and file hashes. Revocation, minimum runtime versions and rollback protection govern acceptance. They do not establish that a module's behavior is safe.

[Native surface limits and capabilities](./sandbox-and-capabilities.md) cover VM memory, stack, call duration, nodes and queued operations, plus managed capabilities such as network destinations, WebView navigation and clipboard access. Configure an explicit policy for restricted modules: an omitted declaration imposes no module-level restriction, and effective access must also satisfy the host’s grant.

A VM is an in-process boundary, not operating-system process isolation. Custom native and business APIs must perform their own authorization, and QuickJS bytecode must come from trusted inputs such as a verified package. Evaluate the complete host and its exposed capabilities when deciding what generated code to accept.

## Choosing NSH

NSH is especially useful when an AI assistant needs to return an actionable interface, a client needs independently delivered modules, or an existing native/Rust application needs a dynamic extension layer. The core choice is to let interfaces arrive at runtime while the host controls their integration and execution. A complete Rust application remains an option when dynamic code is unnecessary.
