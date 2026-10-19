---
title: Modules
description: The modules an NSH app imports, what each one exports, and how they are organized.
order: 1
---

# Modules

An NSH app imports everything it uses from a small set of modules. They are all native: the reactive graph, the renderer and every module are implemented in Rust inside the runtime, and the bundler keeps `import ... from "nsh/..."` as imports instead of copying code into your bundle. Your bundle only holds your own code.

```tsx
import { mount, signal } from "nsh";
import { createNativeValue } from "nsh/animated";
import { windowSize } from "nsh/device";
import { Button, SafeAreaView } from "nsh/ui";
```

## Two layers

- **`nsh` and `nsh/<module>`** are the primitives and the environment: reactivity, lists, canvas, animation, gestures, device information. Every UI set builds on them.
- **`nsh/ui` and `nsh/ui/<set>`** are the UI built on them. `nsh/ui` holds what every form factor shares: navigation, modals, safe areas, alerts and components such as `Button` and `TextField`. `nsh/ui/<set>` adds what one form factor has: `nsh/ui/mobile` holds the status bar, sheets, action sheets and the software keyboard (see [Components](../components/index.md)).

There is no catch-all module: each API lives in exactly one place. Platform-only APIs live in `nsh/ios`, `nsh/android` and `nsh/desktop` (macOS, Windows, Linux); all can be imported on every platform, so one bundle runs everywhere (their calls do nothing elsewhere, and `isAvailable` tells you which platform you are on).

Types are generated from the runtime: `nsh init`, `nsh build`, `nsh dev` and `nsh trace` write `.nsh/types` next to your app, so the TypeScript declarations always match the runtime you build with (see [Project structure](../docs/project-structure.md)).

## All modules

| Module | Exports | Page |
|---|---|---|
| `nsh` | `signal`, `computed`, `effect`, `batch`, `untracked`, `cleanup`, `context`, `split`, `mount`; `View`, `Text`, `Scroll`, `Input`; the types of elements, styles, events and JSX (`YNode`, `ViewStyle`, ...) | [nsh](./nsh.md) |
| `nsh/animated` | `createNativeValue`, `NativeValue`, `NativeNode`, `Animated`, `Easing`, `LayoutAnimation` | [nsh/animated](./nsh-animated.md) |
| `nsh/list` | `List` (virtualized, recycling list) | [nsh/list](./nsh-list.md) |
| `nsh/canvas` | `Canvas`, `Path2D`, `CanvasRenderingContext2D`, `CanvasGradient` | [nsh/canvas](./nsh-canvas.md) |
| `nsh/device` | `windowSize`, `colorScheme`, `fontScale`, `Platform`, `PixelRatio`, `I18nManager` | [nsh/device](./nsh-device.md) |
| `nsh/gesture` | `PanResponder` | [nsh/gesture](./nsh-gesture.md) |
| `nsh/image` | `Image` (the element), `Image.getSize`, `Image.prefetch` | [nsh/image](./nsh-image.md) |
| `nsh/clipboard` | `Clipboard.setString`, `Clipboard.getString`, `Clipboard.hasString` | [nsh/clipboard](./nsh-clipboard.md) |
| `nsh/keymap` | `keymap.bind` (keyboard shortcuts: `mod+k`, `ctrl+shift+p`, ...) | [nsh/keymap](./nsh-keymap.md) |
| `nsh/native` | `nativeComponent` (components the embedding app registers) | [nsh/native](./nsh-native.md) |
| `nsh/accessibility` | `AccessibilityInfo`, `screenReaderEnabled`, `reduceMotion` | [nsh/accessibility](./nsh-accessibility.md) |
| `nsh/ios` | `isAvailable`, `InputAccessoryView` | [nsh/ios](./nsh-ios.md) |
| `nsh/android` | `isAvailable`, `Toast`, `BackHandler`, `setStatusBarBackgroundColor` | [nsh/android](./nsh-android.md) |
| `nsh/desktop` | `isAvailable`, `Window` (title, title bar, maximize, full screen), `titleBar`, `Menu` (application menu, context menus) | [nsh/desktop](./nsh-desktop.md) |
| `nsh/webview` | `WebView` | [nsh/webview](./nsh-webview.md) |
| `nsh/video` | `Video` | [nsh/video](./nsh-video.md) |
| `nsh/ui` | Navigation (`createNavigator`, `Stack`, `Screen`, `Modal`), `SafeAreaView`, `safeArea`, `useSafeAreaInsets`, `Alert`, `Button`, `Card`, `Group`, `ListView`, `SelectList`, `Stepper`, `TextField` | [Components](../components/index.md) |
| `nsh/ui/mobile` | `StatusBar`, `Sheet`, `ActionSheet`, `KeyboardAvoidingView`, `Keyboard`, `keyboard` | [Components](../components/index.md) |
| Globals | `fetch`, `URL`, `TextEncoder`, `Blob`, `FormData`, `AbortController`, `Intl`, timers, `console`, `requestAnimationFrame`, ... (no import) | [Global APIs](./globals.md) |

Reference pages that are not modules:

| Page | Covers |
|---|---|
| [Style reference](./style-reference.md) | Every style key, its type and meaning |
| [Tailwind reference](./tailwind-reference.md) | The Tailwind utilities and variants `class` accepts |

## Host elements

The host elements are components imported like everything else: `View`, `Text`, `Scroll` and `Input` from `nsh`, `Image` from `nsh/image`, `List` from `nsh/list`, `Canvas`, `WebView`, `Video` from their modules, `Modal` from `nsh/ui`, `InputAccessoryView` from `nsh/ios`. Only `If` and `Each` are globals. The compiler makes the elements in place, so the imports cost nothing at run time. Their props are documented under [Elements](../elements/index.md).
