# The native pipeline: nsh's JSX compiler and native modules

Apps opt in with `"compiler": "nsh"` in `nsh.json`. Such an app is one TSX file (more if it likes) importing only `nsh`; the
bundle is one ES module holding only the app's code. The JS runtime (`js/runtime`, Solid) is not in it: the reactive
graph, the renderer and the API are native. This becomes the only pipeline when the rest of the runtime is native (then
`js/runtime`, `js/ui/mobile` and `js/vendor/solid-js` are deleted).

```tsx
import { signal, computed, mount } from "nsh";

function Counter(props: { label: string }) {
  const count = signal(0);
  return (
    <view onPress={() => count.update((c) => c + 1)}>
      <text>{props.label}: {count()}</text>
      <if when={count() > 2} else={<text>keep going</text>}>
        <text>done</text>
      </if>
    </view>
  );
}

mount(() => <counter label="Taps" />);
```

## Modules

`nsh_core::js::NATIVE_MODULES` names them; the bundler leaves imports of them in the bundle (`plugin.rs`), `nsh-js`
serves them (`modules.rs`):

- `nsh`: `signal` (`count()`, `count.set(v)`, `count.update(f)`), `computed`, `effect`, `batch`, `untracked`, `cleanup`,
  `context` (`Theme()`, `<Theme.Provider value>`), `mount`.
- `nsh:rt` (only compiled code imports it): `element`, `text`, `append`, `insert`, `prop`, `bindProp`, `spread`,
  `component`, `cond`, `each`, `merge`, `setup`.

QuickJS-NG resolves imports when it compiles a module, so the bytecode compiler installs the same loader; names are
checked when the module links (evaluation). The exports are native functions made on first import.

## The compiler (`crates/nsh-bundler/src/jsx.rs`)

One Oxc pass removes TypeScript and lowers JSX (`plugin.rs` `compile_native`):

- **Tags.** Lowercase tags are built-ins: the host elements of `jsx::ELEMENTS` (kebab names: `<web-view>`) and `<if when
  else>` / `<each of key>`, unless a capitalized binding in scope has that name in kebab case (`WeatherPanel` is
  `<weather-panel>`, an imported `Button` is `<button>`): the app's bindings win over built-ins. Capitalized and member
  tags name bindings. Unknown tags are errors that name the import when a UI set has the component. Props keep their
  camelCase names.
- **Host elements** become one arrow function making the element and its host children: static props and children
  are set once (`prop`, `append`), changing ones are functions the renderer follows natively (`bindProp`, `insert`).
  An expression changes when it calls something or reads a member outside nested functions (Solid's rule). Event
  handlers are set once when they are a function or a name, followed otherwise. `ref={el}` assigns the variable,
  `ref={fn}` calls it. Consecutive dynamic children share one `insert`, before the next static child.
- **Components** are called untracked with a props object: static props as values, the others as getters, children as
  the `children` getter (Solid's convention); spreads go through `merge`, which keeps getters.
- Code is generated as JS text with holes, parsed, and the app's expressions are moved into the holes: they keep their
  spans (the source map points at the app), generated nodes take their JSX element's span.
- The entry's prologue gives the renderer the app's themes (`setup`).

## The renderer (`crates/nsh-js/src/dom.rs`)

- A node is an object of a native class holding its id, its children's objects and its event handlers, reported by its
  GC mark: the tree and what handlers capture are one heap QuickJS collects. The root view is held by the renderer;
  a node nothing refers to is collected and its view released (its finalizer queues the release separately, as the op
  queue may be borrowed when a finalizer runs).
- Changes are `Op`s pushed straight into the host's queue (no JS op buffer, no decoding of one); props are decoded by
  `protocol::decode_prop` from the JS value. Events of these nodes are dispatched natively (`Host::dispatch_event`).
- Dynamic children are a render effect whose value is the array of nodes it inserted, reconciled with Solid's
  `reconcileArrays` against the renderer's mirror of the tree; a node that comes again later is moved, never removed
  and inserted (the core would remake its view), and an insert that would not move a node is skipped.
- `each` keys rows (`key(item)`, else the item; objects by identity, primitives by value) and updates the item and
  index signals of kept rows; each row has its own owner. `cond` renders a branch per change of truthiness.
- The reactive graph and the renderer share one per-context state (`state.rs`); natives reach each part by shared or
  short exclusive references, never across a call into JS or a free of a JS value.

## Types (`crates/nsh-bundler/src/types.rs`)

`.nsh/types` of a native app has the native `nsh` and a JSX namespace with the built-ins and the app's kebab-case
components (`nsh/jsx-runtime.d.ts`, rewritten after every build): an exported or imported component's props come from
its own type; one that is neither is not checked (TypeScript's intrinsic elements are global, so the compiler stays the
reference for which tag names what). `<each>` items are not typed (intrinsic elements cannot be generic).

## Not there yet

The `nsh/*` runtime modules (`List`, `animated`, `canvas`, gestures and the responder system, device, accessibility,
platform modules), `nsh/ui/mobile` (its Rust components and navigation), node methods (`focus`, `scrollTo`, …) and hot
module replacement (a development build reloads the whole bundle).
