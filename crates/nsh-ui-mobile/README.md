# nsh-ui-mobile

The components of `nsh/ui/mobile`: buttons, cards, switches, sliders, sheets, tab bars and the like, written in Rust.
One implementation draws the same on iOS and Android: a component renders nsh's elements with typed styles and theme
colors, the core lays it out in the same pass as everything else, and its interaction (press feedback, drags, state)
stays native. JS only gets the events the component emits.

- **One file per component** (`src/button.rs`), registered in `registry()` (`src/lib.rs`). Start with
  `use nsh_core::ui::prelude::*;`.
- **Props are a struct** (`#[derive(Clone, PartialEq, Props)]`), **events an enum** (`#[derive(Event)]`, or `NoEvent`),
  choices an enum (`#[derive(PropEnum)]`). The bundler generates the `nsh/ui/mobile` export and its TypeScript
  declarations from them, doc comments included, so apps write `<Button title="Save" onPress={save} />` with full
  types; there is no hand-written glue. The props struct's doc comment documents the component in TypeScript.
- **`render(&self, props, cx) -> impl IntoElement`** builds the tree with `view()`, `text()`, `image()` and other
  components (`Card::element(props)`), styled with GPUI-style methods (`.px_4()`, `.rounded(px(theme.radius))`,
  `.bg(theme.primary)`, `.active(|s| s.bg(theme.primary_active))`). It runs again when a prop changes or a listener
  calls `cx.notify()`; keep it cheap and free of side effects. The root must be a `view()`: it is the component's host,
  so the app's `class` (merged last, it wins) and `style` apply to it.
- **Colors come from the theme** (`cx.theme()`: gpui-kit's semantic fields, a light and a dark value each), never
  literals: apps restyle every component with their `nsh.json` theme, and dark appearance switches natively.
- **What the render knows is plain Rust**: `.when(props.disabled, |v| v.opacity(0.5))`, `match props.variant`.
  Interaction states that change without a render are variants: `.active(..)`, `.hover(..)`, `.focus(..)`.
- **The app's content is props, placed by closures**; the component lays it out, never creates or changes it:
  `header: Option<Slot>` (`.slot(props.header, |h| h.px_4())`), `children: Children` (`.child(props.children)`, or
  `.children_each(props.children, |child, at| ..)` to wrap each), `items: Items` for virtualized rows
  (`.list(props.items, |l| l.estimated_item_size(48.0), |row, at| ..)`; in JS `each` and a row renderer, like `List`).
  `at` tells the index, the count, first and last.
  `Group` and `ListView` are the examples.
- **State** lives in the component's fields, set up from the first props in `setup` and adjusted in `props_changed`;
  listeners (`cx.listener(|this, event, cx| ..)`) get `&mut Self` and emit with `cx.emit(ButtonEvent::Press)`. Timers
  and async work are tasks (`cx.set_interval`, `cx.spawn` with tokio inside); keep the `Task` in a field and drop it to
  stop. Read the environment in the render (`cx.safe_area()`): it renders again when that changes. `Stepper` is the
  example.
- **Primitives only**, never a platform widget: the look is ours and identical on both platforms.
- Changing a component needs a rebuild of the runtime (the dev hosts, `nsh-ios-app`, the Android runtime); apps using
  it reload as usual.

How the core runs components: `docs/dev/components.md`.
