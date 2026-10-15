# nsh-ui-mobile

The components of `nsh/ui/mobile`: buttons, switches, sliders, sheets, tab bars and the like, written in Rust. One
implementation draws the same on iOS and Android: a component renders nsh's primitives (views, texts, images,
other components) with Tailwind classes, the core lays it out in the same pass as everything else, and its
interaction (press feedback, drags, state) stays native. JS only gets the events the component emits.

- **One file per component** (`src/button.rs`), registered in `registry()` (`src/lib.rs`).
- **The spec is its JS face.** `Spec` names the component (`nsh.button`), its JS export (`Button`), its props (type,
  default, doc), its events and whether it takes children. The bundler generates the `nsh/ui/mobile` export and its
  TypeScript declarations from it (`nsh_core::component::typescript`), so apps write `<Button title="Save"
  onPress={save} />` with full types; there is no hand-written glue.
- **`render(&self, props, cx) -> El<Self>`** builds the element tree with `view()`, `text()`, `image()`,
  `component()`: classes (`.class()`, with `active:` / `disabled:` / `dark:` variants), accessibility (`.role()`,
  `.label()`, `.state()`), handlers (`.on_press()`, `.on(kind, …)`) and `.holds_children()` where JSX children go.
  It runs again when a prop changes or a handler calls `cx.notify()`; keep it cheap and free of side effects. The
  root must be a `view()`: it is the component's host, so the app's `class` (appended last, it wins) and `style`
  apply to it.
- **State** lives in the component's fields; handlers get `&mut self`. `cx.emit("press", data)` reaches the app's
  `onPress`.
- **Primitives only**, never a platform widget: the look is ours and identical on both platforms.
- Changing a component needs a rebuild of the runtime (the dev hosts, `nsh-ios-app`, the Android runtime); apps using
  it reload as usual.

How the core runs components: `docs/dev/components.md`.
