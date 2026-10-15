# Components

The components of a UI set (`Button`, `Card`, ... of `nsh/ui/mobile`) are written in Rust and run by the core on the
engine thread. JS only describes the UI: a component is one node in the JSX tree, its props go to Rust, its events come
back. One implementation draws the same on iOS and Android, is laid out in the same Taffy pass as everything else
(no measuring round trip) and keeps its interaction native.

| Where | What |
|---|---|
| `crates/nsh-core/src/ui/` | The API (`nsh_core::ui`, `prelude`): `Component`, elements, `Styled`, `Theme`, `Registry`, the glue generator `typescript` |
| `crates/nsh-macros` | `#[derive(Props)]`, `#[derive(PropEnum)]`, `#[derive(Event)]`; `style_helpers!` (the spacing methods of `Styled`) |
| `crates/nsh-core/src/core/components.rs` | The core side: instances, expansion, diffing, children and slots, events |
| `crates/nsh-ui-mobile` | The mobile set's components, one file each (`README.md`: how to write one) |
| `crates/nsh-bundler/src/embedded.rs` | `ui/mobile/components.gen.ts`, generated from `nsh_ui_mobile::specs()`, re-exported by `nsh/ui/mobile` |
| `js/runtime/_core.ts` | `component(name, props, events, children, slots)`: what the generated glue calls |
| `nsh_engine::EngineConfig::components` | The registry a surface renders from (the platforms and `nsh trace` pass `nsh_ui_mobile::registry()`) |

A desktop set will be a crate of its own (`nsh-ui-desktop`, on GPUI), not an abstraction over both.

## The API

```rust
#[derive(Default)]
pub struct Button;                       // the component's state

/// A press target with a label.         // the TypeScript doc of `Button`
#[derive(Clone, PartialEq, Props)]
pub struct ButtonProps {
    /// The label.
    pub title: SharedString,             // required in TS
    #[prop(default)]
    pub variant: ButtonVariant,          // #[derive(PropEnum)]: "primary" | "secondary" | ...
    pub accessibility_label: Option<SharedString>,   // `accessibilityLabel?`
}

#[derive(Event)]
pub enum ButtonEvent {
    /// The button was pressed.
    Press,                               // `onPress?: () => void`
}

impl Component for Button {
    const NAME: &'static str = "nsh.button";       // JS export: `Button`
    type Props = ButtonProps;
    type Event = ButtonEvent;

    fn render(&self, props: &ButtonProps, cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        let theme = cx.theme();
        view()
            .px_4()
            .py_3()
            .rounded(px(theme.radius))
            .bg(theme.primary)
            .active(|s| s.bg(theme.primary_active))
            .on_press(cx.listener(|_this, _, cx| cx.emit(ButtonEvent::Press)))
            .child(text(props.title.clone()).text_color(theme.primary_foreground))
    }
}
```

- **Props** are a struct, **events** an enum; the derives produce the decoding of `$name` values from JS, defaults,
  and the TypeScript types (doc comments included). Field and variant names become camel case in JS. Value types
  implement `JsValue` (numbers, `bool`, strings, `Color`, `Option`, `Vec`, `JsonValue`, `PropEnum`s). A prop JS sends
  with the wrong type is refused with a warning and nothing renders; clearing a prop restores its default.
- **The app's content is props too, placed by closures**: the component never creates or touches the app's elements,
  it only lays them out.
  - `header: Option<Slot>` is `Some` while the app passes non-empty JSX for it: `.slot(props.header, |h| h.px_4())`
    places it (the closure styles the slot's own node).
  - `children: Children`: `.child(props.children)` puts them all at one place among the render's own children;
    `.children_each(props.children, |child, at| view().py_3().child(child))` wraps each one (it runs again when
    the children change; `at` is a `Position`: `index`, `count`, `is_first()`, `is_last()`, what a Solid row knows
    from its index and the data).
  - `items: Items`: virtualized rows. In JS the component takes `List`'s data props (`each`, the row renderer as
    children, `keyExtractor`, `onEndReached`, `ListHeaderComponent`, ... : `ListItemsProps<T>`, so the component is
    generic over the row type). `.list(props.items, |l| l.flex_1().estimated_item_size(48.0), |row, at|
    view().px_4().when(at.is_last(), |v| v.rounded_b(px(16.0))).child(row))` places the list, lays it out (size, columns, padding, gaps, direction) and wraps each
    row; only rows near the viewport exist and cells are recycled as with `List`.
- **Elements** are `view()`, `text(..)`, `image(..)`, strings (as texts) and other components (`Button::element(props)`,
  whose `.on_event(cx.listener(..))` handles their typed events instead of the app). Their abilities are traits:
  `Styled`, `Accessible`, `Interactive` (`on_press`, `active` / `hover` / `focus` styles, `disabled`), `ParentElement`,
  `FluentBuilder` (`when`, `when_some`, `map`).
- **State** is `Self`, written like a Solid component where Rust allows:

  | Solid | Rust component |
  |---|---|
  | the component's body (runs once) | `fn setup(&mut self, props, cx)` |
  | an effect `on` a prop | `fn props_changed(&mut self, old, new, cx)` (before the next render) |
  | event handlers | `cx.listener(|this, event, cx| ..)`: `&mut Self`, `cx.emit(..)`, `cx.notify()` renders again |
  | `setTimeout` / `setInterval`, `clear*` | `cx.set_timeout(d, ..)` / `cx.set_interval(d, ..)` return a `Task`; dropping it clears it |
  | async work, `onCleanup` | `cx.spawn(|this, cx| async move { .. this.update(..).await })`; `Drop`, and the component's tasks end with it |
  | `windowSize()`, `safeArea()`, `keyboard()`, `colorScheme()`, ... | `cx.window()`, `cx.safe_area()`, `cx.keyboard_height()`, `cx.dark()`, ... in the render: reading subscribes |

  `render(&self)` cannot change state. Timers are tasks: a `tokio::time` sleep, then an update.

## Tasks

`ui/task.rs`: a small executor of the core runs components' futures on the engine thread, so they may hold the
component's `!Send` state. Each poll runs inside a tokio runtime's context (`Core::set_task_runtime`; the engine's
`EngineConfig::runtime`, else one runtime of two threads shared by the process), so the tokio ecosystem works in a
task: `tokio::time` timers, network and file IO, channels; `cx.background(fut)` / `cx.blocking(f)` run `Send` work on
the runtime's threads. A waker may fire on any thread: it sends the engine a `Wake` message, and the loop runs the woken
tasks (`Core::run_component_tasks`). `this.update(f)` does not touch the component during the poll: the core applies
the queued updates after it (with a `Context`, so they can `notify`, emit, spawn), then the tasks waiting for their
results run on; it resolves to `None` when the component is gone. A `Task` dropped cancels its future; a released
component cancels all of its own.

## Environment

The engine hands the core what components may read (`ui::Environment`: window, safe area, keyboard height, appearance,
font scale, reduced motion, screen reader, direction). A render records what it read (`RenderCx::window()` and the
like); when the environment changes, only the components that read a changed part render again.

## Styles

`Styled` methods are named as in GPUI and Tailwind (`.flex_row()`, `.items_center()`, `.p_4()` = 16pt, `.gap(px(6.0))`,
`.rounded_xl()`, `.border_1()`, `.shadow_md()`, `.text_sm()`, `.font_weight(FontWeight::SEMIBOLD)`, ...); spacing steps
follow Tailwind's default scale, independent of the app's Tailwind theme. A render records declarations
(`StyleRefinement`); the core lowers them into the same rules Tailwind classes compile to (`ClassList`), cached and
shared between elements with the same styles, and only when an element's styles changed. There is no string to
parse and no `.class()`.

- Theme colors are `AdaptiveColor`s (a light and a dark value) and lower into one declaration that picks by
  appearance (`Decl::Adaptive`): the platform's appearance switch restyles natively, without a render, and the app's
  plain classes still win over it.
- `.active(|s| ..)`, `.hover(..)`, `.focus(..)` lower into rules on the view's interaction states (Tailwind's ranks):
  press feedback is native.
- What a render knows (a variant, `disabled`) is plain Rust: `.when(props.disabled, |v| v.opacity(0.5))`.
- The app's `class` / `classList` on a component (Tailwind) is merged after the root's rules; at equal rank it wins.
  The styles of a nested component element merge the same way.

## The theme

`Theme` (`ui/theme.rs`) has gpui-kit's semantic colors (`primary`, `primary_active`, `primary_foreground`,
`secondary`, `danger`, `muted`, `border`, `group_box`, `list_*`, `switch*`, `tab*`, ...: the fields mobile components
use) plus `radius`, `radius_lg` and `font_size`, and derefs to its colors (`theme.primary`). The default
(`default-theme.json`) is gpui-kit's default theme with mobile radii. An app sets its own in `nsh.json`: `"theme":
"brand.json"` (a gpui-kit theme file, relative to `nsh.json`) or the same JSON inline. Keys only desktop components
use are ignored; a pressed color the theme leaves out is derived from its base; colors are CSS colors or Tailwind
palette names. The bundler checks the theme and serves it as `nsh:theme`; `_core.ts` sends it to the root
(`uiTheme`) before anything renders; a new theme renders every component again.

## The host node and ids

In JS a component is a `<view>` with a `component` prop naming it (`nsh.button`): its **host**. `Core::apply` sees the
prop, creates an instance and from then on intercepts the ops on the host (`intercept`): the host is the root of the
component's expansion, the elements its render returns below the root become nodes the core creates itself. Apps
without components pay one check per op.

JS allocates even node ids (from 2), the core odd ones (from 3), so both id spaces stay dense and never collide. JS
never sees the expanded nodes; their events are the component's (below).

## Renders and diffing

- A prop change marks the instance; **renders run at the start of `commit`**, so props set in one JS turn render once.
  A component that keeps asking for renders is cut off after 16 per commit.
- The render is diffed against the previous one: keyed elements match by key; unkeyed ones take the next unused old
  element of the same kind in order, so an element that comes and goes (`.children(option)`) does not move its
  siblings onto new nodes. Attributes are compared field by field (`Attrs`), styles as a whole; native children are
  moved only when their order changed.
- An unknown name logs a denial and renders nothing. Rust components take no `ref.command(...)`.

## Children, slots and rows

- JS inserts a component's children into the host; the core places them where the render's `Children` is, before the
  rendered node that follows it (or keeps them out of the tree while the render has none). With `children_each`, a
  change of the children renders the component again: one wrapper per child, keyed by the child's node, so a child
  added or removed leaves the others' wrappers in place.
- Rows: the glue makes a `List` of the app's data and row renderer and hands it to the host as the slot of the
  `Items` prop. The render places it like a slot and sends its layout (`ListConfig`: estimated size, columns,
  direction, padding, gaps). Each item cell of that list becomes a small instance whose render is the component's row
  function (`Inner::Row`): the wrapper goes into the cell, the row the app rendered into the wrapper. When the list
  rebinds a cell to another row (`cellIndex`), its wrapper renders again with the new position; so do all live cells
  when the number of rows changes (the last row moves), and when the component renders again. Binding, recycling, measuring and positioning stay the list's: a rebinding frame
  of `ListView` costs what one of `List` does (5 ops / 4 mount items in the gallery's Mobile UI demo).
- Named slots: the glue creates one holder view per slot **the app passes** (`componentSlot`), filled with that prop's
  JSX. The render's `Slot` element *is* that holder: its styles and handlers apply to it, the core places it but never
  releases it (it is JS's). Filling or emptying a holder renders the component again.

## Events

- Handlers sit on the elements; the native event of an expanded node, or of the host, goes to the component first
  (`Core::component_event`, called by the engine before JS). Events of expanded nodes never reach JS.
- `cx.emit(event)` sends a `NativeEvent` on the host (`{ name, data }`, from the `Event` derive); the glue calls the
  app's `on<Name>(data)`. A nested component's events go to the `on_event` listener of the component that rendered it,
  typed, instead.

## The glue

`nsh_core::ui::typescript(specs)` writes `components.gen.ts`: the props interfaces and one `component(...)` call per
component. That helper runs for every instance, so it is kept minimal: **one render effect reading every prop
directly** and sending the ones that changed, no `splitProps` / `mergeProps` / `spread` (their proxies made the first
version cost more JS than the TSX component it replaced), no node for an absent slot, no event listener when no handler
is passed. Benchmarks against the same Button in TSX: `docs/dev/performance.md` (`examples/bench`, `component_bench`).

## Native components of the embedder

`nsh/native`'s `nativeComponent` is the other kind: a platform view the embedding app registered (Swift / Java
`NshComponent`, `docs/dev/embedding.md`). Those are views of their own on the main thread; Rust components are
compositions of primitives in the core.
