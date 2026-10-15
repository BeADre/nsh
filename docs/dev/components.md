# Components

The components of a UI set (`Button`, `Card`, ... of `nsh/ui/mobile`) are written in Rust and run by the core on the
engine thread. JS only describes the UI: a component is one node in the JSX tree, its props go to Rust, its events come
back. One implementation draws the same on iOS and Android, is laid out in the same Taffy pass as everything else
(no measuring round trip) and keeps its interaction native.

| Where | What |
|---|---|
| `crates/nsh-core/src/component.rs` | The API: `Component`, `Spec`, `Props`, `El` builders, `Registry`, the glue generator `typescript` |
| `crates/nsh-core/src/core/components.rs` | The core side: instances, expansion, diffing, children and slots, events |
| `crates/nsh-ui-mobile` | The mobile set's components, one file each (`README.md`: how to write one) |
| `crates/nsh-bundler/src/embedded.rs` | `ui/mobile/components.gen.ts`, generated from `nsh_ui_mobile::specs()`, re-exported by `nsh/ui/mobile` |
| `js/runtime/_core.ts` | `component(name, props, events, children, slots)`: what the generated glue calls |
| `nsh_engine::EngineConfig::components` | The registry a surface renders from (the platforms and `nsh trace` pass `nsh_ui_mobile::registry()`) |

A desktop set will be a crate of its own (`nsh-ui-desktop`, on GPUI), not an abstraction over both.

## The host node and ids

In JS a component is a `<view>` with a `component` prop naming it (`nsh.button`): its **host**. `Core::apply` sees the
prop, creates an instance and from then on intercepts the ops on the host (`intercept`): the host is the root of the
component's expansion, the elements its render returns below the root become nodes the core creates itself. Apps
without components pay one check per op.

JS allocates even node ids (from 2), the core odd ones (from 3), so both id spaces stay dense and never collide. JS
never sees the expanded nodes; their events are the component's (below).

## Props and renders

- Own props arrive as `$name` (`Prop::ComponentProp`, any JSON value; `undefined` clears). `Props` stores them by
  their index in the spec, with the spec's defaults parsed once per registered component; strings are shared, so
  reading one in a render is a reference count bump.
- A prop change marks the instance; **renders run at the start of `commit`**, so props set in one JS turn render once.
  A component that keeps asking for renders (`notify` from a render's side effects) is cut off after 16 per commit.
- The render is diffed against the previous one: elements match by key, else by position and type; attributes field by
  field (`Attrs`), so only changed props become ops; native children are moved only when their order changed. Class
  runs (`.class("px-4 py-3")`) are kept as static strings and joined only when they change.
- The app's `class` / `classList` on a component is appended to the root's classes (it wins); `style`, `testID` and
  `onLayout` apply to the host as on any view; `ref` gets the host (`ref.command(name, args)` reaches `command`).
- An unknown name logs a denial and renders nothing.

## Children and slots

- `.holds_children()` marks where JSX children go. JS inserts them into the host; the core places them into that
  element (or keeps them out of the tree while the render has no such element).
- Named slots (`Spec::slots`, `<Card header={…}>`): the glue creates one holder view per slot **the app passes**
  (`componentSlot`), filled with that prop's JSX. The render's `.slot(name)` element *is* that holder: its classes and
  handlers apply to it, the core places it but never releases it (it is JS's). `cx.has_slot(name)` tells whether the
  app passed one with content; filling or emptying a holder renders the component again.

## Events

- Handlers sit on the elements (`.on_press(…)`, `.on(kind, …)`); the native event of an expanded node, or of the host,
  goes to the component first (`Core::component_event`, called by the engine before JS). Events of expanded nodes never
  reach JS.
- `cx.emit(name, data)` sends a `NativeEvent` on the host; the glue calls the app's `on<Name>(data)`. A nested
  component's emits go to the component that rendered it instead. `cx.notify()` renders again.

## The glue

`nsh_core::component::typescript(specs)` writes `components.gen.ts`: the props interfaces (from `PropType`, docs from
the spec) and one `component(...)` call per spec. That helper runs for every instance, so it is kept minimal: **one
render effect reading every prop directly** and sending the ones that changed, no `splitProps` / `mergeProps` /
`spread` (their proxies made the first version cost more JS than the TSX component it replaced), no node for an
absent slot, no event listener when no handler is passed. Benchmarks against the TSX Button: `docs/dev/performance.md`
(`examples/bench`, `component_bench`).

## Native components of the embedder

`nsh/native`'s `nativeComponent` is the other kind: a platform view the embedding app registered (Swift / Java
`NshComponent`, `docs/dev/embedding.md`). Those are views of their own on the main thread; Rust components are
compositions of primitives in the core.
