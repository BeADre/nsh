# JS modules

Two layers. `nsh` and `nsh/<module>` (`js/runtime`) are the primitives and the environment that every UI set builds
on. `nsh/ui/<set>` (`js/ui/<set>`) is a whole UI for one form factor: `nsh/ui/mobile` has everything that is mobile UI or
interaction (navigation stacks, modals, the status bar, system dialogs, and its components, one look on iOS and
Android, written in Rust in `crates/nsh-ui-mobile`); a desktop set will be a separate one (`nsh/ui/desktop`), not an abstraction over both.
Shared vs one form factor decides the layer, not JS vs native.

`nsh-bundler/src/embedded.rs` lists the embedded files per root (`runtime`, `ui/mobile`) with the specifiers apps import;
files import their own root's files relatively (`./navigation`) and the runtime's internals as `nsh/_core` (embedded
files only; apps cannot). `.nsh/types` has one declaration file per embedded file (`nsh/list.d.ts`,
`nsh/ui/mobile/navigation.d.ts`) and `nsh/ui/mobile.d.ts` re-exporting its set, found through the `nsh/*` path of the
app's `tsconfig.json`; files nsh no longer generates are removed. `ui/mobile/components.gen.ts` is generated at build
time from the Rust components' specs (`docs/dev/components.md`); so are the runtime's Rust modules
(`nsh_core::runtime`): `runtime/<module>.gen.ts`, which the hand-written module re-exports and adds its JS part to
(`nsh/android`: the back button's listeners), or the whole module when nothing in it is JS (`nsh/image`). `nsh` itself only has rendering and the types; there
is no catch-all module, and moved APIs get no compatibility re-exports.

| Module | Exports |
|---|---|
| `nsh` | `render`, `root`, `YNode`; the types of styles, element props, events and `JSX` |
| `nsh/animated` | `createNativeValue`, `NativeValue`, `NativeNode`, `Animated`, `Easing`, `LayoutAnimation` |
| `nsh/device` | `windowSize()`, `colorScheme()`, `fontScale()`, `Platform`, `PixelRatio`, `I18nManager` |
| `nsh/accessibility` | `AccessibilityInfo`, `screenReaderEnabled()`, `reduceMotion()` |
| `nsh/list` | `List` |
| `nsh/canvas` | `Canvas`, `Path2D`, `CanvasRenderingContext2D`, `CanvasGradient` |
| `nsh/image` | `Image.getSize`, `Image.prefetch` (Rust: promises the core resolves from the root's `ImageInfo` event) |
| `nsh/gesture` | `PanResponder` |
| `nsh/webview`, `nsh/video` | `WebView`, `Video` |
| `nsh/native` | `nativeComponent(name, { props, events })`: a component for a native component the embedding app registered |
| `nsh/ios` | `isAvailable`, `InputAccessoryView` |
| `nsh/android` | `isAvailable`, `Toast.show(message, "short" \| "long")`, `BackHandler` (`addEventListener` in JS, `exitApp` Rust), `setStatusBarBackgroundColor` (the status bar's base layer) |
| `nsh/renderer` | what compiled JSX calls (`createElement`, `insert`, `spread`, ...; the Solid compiler's `moduleName`) |
| `nsh/ui/mobile` | `Stack`, `Screen`, `createNavigator`, `Modal` (Solid, `navigation.ts`); everything else is Rust in `nsh-ui-mobile`, its JS generated: `SafeAreaView`, `KeyboardAvoidingView`, `StatusBar` (and `StatusBar.setBarStyle` / `setHidden`), `Alert.alert` and `ActionSheet.show` (promises of the index chosen; a list dialog on Android), `Keyboard.dismiss`, `Button`, `Card`, `Group`, `ListView`, `Stepper`, `TextField`, `Sheet`, `SelectList`; re-exported from the runtime: `safeArea()`, `keyboard()`, `useSafeAreaInsets()` |

- **Platform modules** are importable on both platforms (one bundle runs on both); on the other one their calls do
  nothing and their components render nothing. `isAvailable` tells which.
- **Platform-only props** are grouped: `ios={{…}}` / `android={{…}}` / `desktop={{…}}` (macOS, Windows, Linux) on
  elements and components (`IOSProps` / `AndroidProps` / `DesktopProps` in `types.ts`, extended per element; the host props in them are the
  generated `IOS…HostProps` / `Android…HostProps` of `elements.gen.ts`). The
  renderer (`platformProps` in `_core.ts`) sends only the current platform's object, each key as a prop of its own (keys removed from the object are cleared); the core knows them by
  those names. Android: `ripple`, `elevation`, `accessibilityLiveRegion`, Modal `statusBarTranslucent`, StatusBar
  `backgroundColor` (a Rust component's `android` prop). Alert's `cancelable` is an option of both platforms (iOS
  alerts are never dismissed by a tap outside). iOS: input `inputAccessoryViewID` / `textContentType`, Screen `headerLargeTitle` /
  `headerBackTitle`, Modal `presentationStyle` / `supportedOrientations`, WebView `allowsBackForwardNavigationGestures`.
- Module-level code that must always run (the engine's hooks `__nsh_dispatch` / `__nsh_env`, the root's direction, the
  Tailwind theme) lives in `_core.ts`; feature modules only run when imported (production bundles tree-shake the rest).
  `_core.ts` recognizes native animation values by their `__nv` field, so it does not import `nsh/animated`.

## Where new things go

- A primitive or environment both UI sets need: a module of `js/runtime` (`nsh/<module>`), listed in
  `crates/nsh-bundler/src/embedded.rs`. Functions that only ask the platform (a command, a request answered by an
  event) are a Rust `#[module]` in `crates/nsh-core/src/runtime/`, registered in `runtime::registries()` under the
  module's specifier; only what has to call the app's JS (listeners) or read JS signals synchronously is TS.
- Mobile UI or interaction: `js/ui/mobile` (`nsh/ui/mobile`); its components are Rust, in `crates/nsh-ui-mobile`
  (see its README), with generated glue.
- Something only one platform has: `nsh/ios` / `nsh/android`, or a prop in `ios={{…}}` / `android={{…}}` /
  `desktop={{…}}`.
- Never a compatibility re-export when something moves, never a React Native compatibility wrapper.
