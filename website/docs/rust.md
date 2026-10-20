---
title: Building with Rust
description: "Write a Native Shell app entirely in Rust, with no QuickJS on any platform, or in Rust with TSX screens on top: when to choose which, and how the two share components and modules."
order: 5.5
group: start
---

# Building with Rust

Native Shell apps are written in TSX, in Rust, or in both. All three run on the same engine: the reactive graph, the
renderer, Taffy layout and the native modules (navigation, dialogs, the keyboard, the status bar) are Rust in every case, and every platform renders with its
own UI. What differs is the language of your own code, and with it whether the app carries a JavaScript VM.

| | TSX | Rust | Rust with TSX |
|---|---|---|---|
| Your UI, state and handlers | TSX on QuickJS | Rust | Rust components and modules, TSX screens that use them |
| The app runs | QuickJS and the engine | the engine and your crate only, on every platform and in the browser | QuickJS and the engine, with your crate |
| Suits | Apps and modules delivered at run time, quick iteration | Medium and large apps with demanding business logic | A Rust core with parts that change often |
| Edits | Hot updates | Rebuild and restart | Hot updates for TSX, rebuild and restart for Rust |
| Start | `nsh init my-app` | `nsh init my-app --rust` | `nsh init my-app --rust`, then `nsh run App.tsx` |

## Choosing

**Rust** suits apps whose business logic is large and demanding: trading, editors, data-heavy tools. You get Rust's
type system and compile-time checks across the whole app, the crates.io ecosystem, native performance for your own
code (not only for NSH's), and headless tests of the real UI with `cargo test`. The app is Rust and native code from
start to end: no JavaScript VM ships with it or runs in it, on iOS, Android, macOS, Windows, Linux, GPUI and the web.
Components are functions that run once and bind signals to what they show, as in TSX ([Reactivity](./reactivity.md)),
written with typed styles instead of class strings.

**Rust with TSX** keeps that core and adds TSX where it pays off: screens, settings, onboarding, experiments,
anything you iterate on with hot updates or deliver at run time as a [signed package](./signed-packages.md). TSX
imports your Rust components and modules like any NSH module, so the two halves meet at typed interfaces. The TSX
runs on QuickJS, which the app then carries.

## An app written in Rust

```sh
nsh init my-app --rust
cd my-app
nsh run                        # this machine's backend; --target ios, android, web, macos, linux, windows or gpui
nsh package --target web       # or ios, ios-simulator, android, macos, linux, windows, gpui
```

`src/lib.rs` holds the app: components, state, handlers, tasks.

```rust
use nsh_core::ui::prelude::*;

/// The components available to TSX through `app/native`.
#[native_module("app/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

#[component]
pub fn App() -> impl IntoElement {
    let count = signal(0_u32);
    let quote = signal(SharedString::from("…"));
    // Async work runs in tasks; requests go through NSH's network stack.
    spawn(async move {
        if let Ok(response) = fetch("https://api.example.com/quote").await {
            quote.set(response.text().into());
        }
    });
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .gap_4()
        .child(text(move || format!("Count: {}", count.get())))
        .child(text(quote))
        .child(
            // A button is a view that answers presses, drawn with the theme's colors.
            view()
                .px(px(20.0))
                .py(px(12.0))
                .rounded(move || px(theme().radius))
                .bg(move || theme().primary)
                .active(|s| s.bg(move || theme().primary_active))
                .role(AccessibilityRole::Button)
                .on_press(move |_: &()| count.update(|c| *c += 1))
                .child(text("Tap").text_color(move || theme().primary_foreground)),
        )
}
```

`nsh.json` names the root, and `nsh` mounts it:

```json
{
  "name": "My App",
  "rust": ".",
  "root": "App",
  "theme": "theme.json",
  "appearance": "system"
}
```

`"root"` is the component's path in the crate (`App`, `shell::Gallery`; in the first one with
[several crates](#several-crates)); `"theme"` is the [theme](#theme) its components draw with, and `"appearance"` applies to it as it does to a TSX app. With it,
`nsh run` and `nsh package` of the app's directory build the app's own runner for the target, with the root mounted
and no QuickJS linked in:

| Target | What runs |
|---|---|
| macOS, Windows, Linux, GPUI | The release runner with your crate, in a window; `nsh run` runs a debug build and shows the app's logs in the terminal |
| iOS, the iOS simulator, Mac Catalyst | The iOS runner with your crate, in an app |
| Android | The app's `libnsh.so` with your crate, started by the runtime's activity |
| Web | `nsh.wasm` with the engine and your crate (3.3 MB for the whole Rust gallery, 0.75 MB with brotli); the page mounts the root |

During `nsh run`, a change to the crate's Rust code builds the runner again and restarts the app; in the browser,
the open page loads the new runtime by itself. `cargo run` starts the same root directly on the desktop backend
(`src/main.rs`), and native apps that embed NSH mount a Rust root with `AppSource::Rust` on the `AppView` of macOS,
Windows, Linux, GPUI and iOS.

Code for every platform, the web included, uses NSH's services: `fetch` for HTTP
([Networking](./networking.md#from-rust-components)), `timer`, `set_timeout` and `set_interval` for waiting,
`nsh_core::time::Instant` and `nsh_core::time::unix_ms()` for the clock. `background` and `blocking` run work on the
native targets' threads.

## Desktop title bars

Rust apps draw their own title bars with `nsh_core::ui::desktop`. Call
`set_title_bar(TitleBarStyle::Hidden, Some(height))`, mark the bar's background
with `.window_drag(true)`, and reserve the native window controls' space with
`title_bar()`. Its `top` is the bar's height, and `left` and `right` are the widths
occupied by the system controls. Reading it subscribes to changes, including full
screen. The platform handles window dragging and the user's title-bar double-click
behavior; `.window_drag(false)` keeps a child out of its parent's drag region.

```rust
use nsh_core::{
    desktop::TitleBarStyle,
    ui::{desktop::{close_window, set_title_bar, title_bar}, prelude::*},
};

#[component]
fn TitleBar() -> impl IntoElement {
    set_title_bar(TitleBarStyle::Hidden, Some(42.0));
    view()
        .h(px(42.0))
        .flex_row()
        .items_center()
        .window_drag(true)
        .pl(move || px(title_bar().left + 12.0))
        .pr(move || px(title_bar().right + 12.0))
        .child(text("My app").flex_1())
        .child(text("Close").window_drag(false).on_press(|_: &()| close_window()))
}
```

`set_title_bar(TitleBarStyle::Default, None)` restores the system title bar.
`close_window()` follows the embedding host's normal window-close behavior.

## Extending a Rust app with TSX

The crate's `#[native_module("app/native")]` function names what TSX may import: components
(`registry.register::<Chart>()`), modules of functions (`registry.module::<Portfolio>()`, a `#[module]` `impl`),
classes (`registry.class::<Ledger>()`, `#[class]` on a struct and its `impl`) and assets. Their TypeScript
declarations, doc comments included, are generated from the Rust types into `.nsh/types`, so TSX is checked against
them by `nsh check` and your editor:

```rust
/// A ledger kept in Rust.
#[class]
pub struct Ledger {
    entries: Vec<f64>,
}

#[class]
impl Ledger {
    /// An empty ledger.
    pub fn new() -> Self {
        Ledger { entries: Vec::new() }
    }

    /// Adds `amount`; the new balance.
    pub fn add(&mut self, amount: f64) -> f64 {
        self.entries.push(amount);
        self.balance()
    }

    /// The balance.
    #[getter]
    pub fn balance(&self) -> f64 {
        self.entries.iter().sum()
    }

    /// Uploads the entries; resolves when the server has them.
    pub async fn upload(this: Instance<Ledger>, url: String) -> Result<(), String> {
        let body = this.with(|l| format!("{:?}", l.entries));
        fetch(HttpRequest::post(url, body)).await.map(|_| ())
    }
}

pub struct Portfolio;

#[module]
impl Portfolio {
    /// The value of the holdings.
    pub fn total() -> f64 {
        1250.0
    }
}
```

```tsx
import { mount, signal, View, Text } from "nsh";
import { Chart, Ledger, Portfolio } from "app/native";   // Rust

const ledger = new Ledger();   // its state lives in Rust

function Overview() {
  const balance = signal(ledger.add(Portfolio.total()));   // Rust, called at once
  return (
    <View className="p-4 gap-2">
      <Text>Balance: {balance()}</Text>
      <Chart symbol="NSH" range="1M" onSelect={(e) => balance.set(ledger.add(e.price))} />
      <View onPress={() => ledger.upload("https://example.com/ledger")} accessibilityRole="button" class="rounded-lg bg-blue-600 px-4 py-2 active:bg-blue-700">
        <Text class="text-white">Upload</Text>
      </View>
    </View>
  );
}

mount(() => <Overview />);
```

Props are the fields of the component's `Props` struct and events the variants of its `Event` enum (`onSelect`),
both in camel case. A Rust component's own content, events and state stay in Rust; TSX passes props in and hears
events. A prop of the wrong type (`level={1.5}` for a `u32` prop) is a warning, as on a built-in element: the
prop keeps its value and the rest of the component renders.

A function or a member runs the way its Rust signature says. One that is not `async` runs inside the TSX call and
returns its value (`Portfolio.total()` is a number, `ledger.balance` a property): it computes and keeps state. An
`async fn` returns a Promise and runs as a task with NSH's services (`fetch`, `timer`, dialogs); a module function
taking `cx: &mut ModuleCx` reaches the platform in order with the UI (a toast, the status bar) and returns a Promise
of its value. Arguments and results are typed from the signature: numbers, strings, `Option`, `Vec`, your
`#[derive(JsObject)]` structs and `#[derive(PropEnum)]` enums, instances of your classes, and TSX functions
(`Callback<A, R>`, which Rust calls at once inside a call, or later with `post` and `call_async`). An argument of
the wrong type throws a `TypeError` in the call, or rejects the Promise of one that returns a Promise; an `Err`
throws (or rejects) with its message. A class's object owns its Rust value: it is freed when the object is collected, or at
once with `ledger.dispose()` (also `using ledger = new Ledger()`). Name the TSX entry to run it, `nsh run App.tsx` or `nsh package App.tsx --target …`: TSX edits are hot
updates, Rust edits rebuild and restart the app.

`examples/gallery-rs` in the repository is a complete Rust app with both entries: every demo is Rust,
`nsh run examples/gallery-rs --target web` (or `ios`, `android`, …) runs its root, `cargo run -p gallery-native`
starts it on the desktop backend, and `nsh run examples/gallery-rs/App.tsx` runs it under its TSX entry.

## Theme

The look of an app is the app's own: NSH provides the primitives and the unstyled base components of `nsh_base`, and
your crate is the styled layer on top, the app's own component library ([Three layers](./architecture.md#three-layers)). Rust components take their colors, corner radii and base font size from a theme, `theme()` in
`nsh_core::ui::prelude`, so one file restyles every component of the app. Each color has a light and a dark value;
when the system switches appearance, a binding such as `.bg(move || theme().primary)` changes color natively, with no
render.

The default theme is gpui-kit's default theme, a neutral black-and-white palette, with mobile corner radii. To use
your own, set `"theme"` in `nsh.json` to the path of a theme file, relative to `nsh.json`, or to the theme's content
inline:

```json
{
  "name": "My App",
  "rust": ".",
  "theme": "brand-theme.json"
}
```

The format is gpui-kit's theme file format. Each entry in `themes` sets the values for one appearance (`mode`:
`"light"` or `"dark"`; an entry without `mode` is light):

```json
{
  "themes": [
    {
      "mode": "light",
      "radius": 8,
      "radius.lg": 14,
      "font.size": 16,
      "colors": {
        "primary.background": "#7c3aed",
        "primary.foreground": "white",
        "border": "neutral-200"
      }
    },
    {
      "mode": "dark",
      "colors": {
        "primary.background": "#a78bfa",
        "primary.foreground": "neutral-950"
      }
    }
  ]
}
```

- Your theme is applied over the default theme, so you list only what you change.
- Colors are CSS colors (`#7c3aed`, `rgb(…)`, `white`) or names from Tailwind's default palette (`neutral-900`,
  `red-500`). The colors of `nsh.json`'s `tailwind` key belong to `class` strings.
- `radius` (`theme().radius`) is the corner radius of controls, `radius.lg` (`theme().radius_lg`) that of large
  surfaces such as cards and sheets, and `font.size` (`theme().font_size`) the base font size. All are in points.
- A pressed color you leave out (`primary.active.background`, `secondary.active.background`,
  `danger.active.background`, ...) is derived from its base color: 12% darker in light mode, 12% lighter in dark
  mode.
- Keys for gpui-kit's desktop components (sidebars, title bars, ...) are accepted and left aside, so a gpui-kit theme
  file works as it is.
- The bundler checks the theme when it builds: a missing file, invalid JSON, an unknown `mode` or a value that is not
  a color fails the build with an `nsh.json: theme: …` error.

The theme's keys, with their default light and dark values:

| Key | Light | Dark | Role |
|---|---|---|---|
| `background` | `white` | `neutral-950` | The background of pages, lists and inputs |
| `foreground` | `neutral-950` | `neutral-50` | Text on `background` |
| `border` | `neutral-200` | `neutral-800` | Dividers and borders |
| `primary.background` / `.active.background` / `.foreground` | `neutral-900` / `neutral-950` / `neutral-50` | `neutral-50` / `neutral-200` / `neutral-900` | The main action (a primary button), pressed, and the text on it; selections and check marks |
| `secondary.background` / `.active.background` / `.foreground` | `neutral-200` / `neutral-300` / `neutral-900` | `neutral-800` / `#212121` / `neutral-50` | Secondary actions, pressed, and the text on them |
| `danger.background` / `.active.background` / `.foreground` | `red-500` / `red-600` / `neutral-50` | `red-400` / `red-500` / `red-600` | Destructive actions, pressed, and the text on them |
| `accent.background` | `neutral-100` | `neutral-800` | A highlighted item, such as a plain button while pressed |
| `muted.background` | `neutral-100` | `neutral-800` | Quiet surfaces: an inset list's page, a grabber, a clear button |
| `muted.foreground` | `neutral-500` | `neutral-400` | Secondary text: captions, labels, placeholders |
| `group_box.background` / `.foreground` | `#f5f5f5` / `#171717` | `neutral-950` / `neutral-50` | Grouped surfaces such as cards, and the text on them |
| `input.border` | `neutral-200` | `#2f2f2f` | The border of inputs |
| `ring` | `neutral-400` | `neutral-500` | The border of a focused input |
| `list.active.background` | `#bfdbfe33` | `#1e40af33` | A list row while pressed |
| `overlay` | `#0000000d` | `#00000033` | The backdrop behind sheets and dialogs |

## Several crates

An app's Rust code can be split into crates, each imported by its own specifier: `"rust"` takes a list of crate
paths, and each crate in it has its own `#[native_module]` function.

```json
{
  "rust": ["native/charts", "native/storage"],
  "root": "App"
}
```

```tsx
import { Chart } from "app/charts";        // native/charts: #[native_module("app/charts")]
import { Store } from "app/storage";       // native/storage: #[native_module("app/storage")]
```

Every runner and host links all of them, and `.nsh/types` declares each specifier. Crates share code through Cargo
path dependencies (`shared = { path = "../shared" }`): a crate of company components can be used by several crates
and listed by several apps, and only the crates listed in `"rust"` export to TSX. Each listed crate has a package
name and a specifier of its own; `nsh` reports a repeated one with the two paths that share it. A Rust app's
`"root"` is a component of the first crate listed. During `nsh run` and `nsh dev`, a change to any of them, or to a
crate they depend on by path, rebuilds the app and restarts it.

## See also

- [Project structure](./project-structure.md): every key of `nsh.json`.
- [Web](./web.md#apps-with-rust-components): Rust apps in the browser.
- [Networking](./networking.md#from-rust-components): `fetch` from Rust.
- [CLI](./cli.md#nsh-init): `nsh init --rust`.
