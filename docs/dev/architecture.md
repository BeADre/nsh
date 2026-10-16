# Architecture

## Data flow and threads

```
JS (engine thread, QuickJS)  components on the native reactive graph -> js/runtime pushes changes into one flat array -> __nsh_commit(ops) in a microtask
nsh-js                        decodes the op buffer in Rust (tags, style objects and event names are parsed here) -> Vec<Op>
nsh-core (engine thread)      apply(Op) maintains the shadow tree -> commit(): render components, rebuild text, Taffy layout, diff frames -> MountBatch
Platform (main thread)       one main-queue hop per commit, applying MountItems to native views in order
Input                        native gestures -> EngineControl::dispatch_event -> engine thread -> JS handler
```

- The platform main thread never waits for JS.
- **Reactivity is native**: signals, memos, effects, owners and context are a graph in Rust (`crates/nsh-js/src/reactive.rs`,
  Solid 1.9's algorithm) whose nodes are QuickJS objects of native classes, so the QuickJS GC sees the cycles through them.
  JS reaches it through `globalThis.__nsh_reactive`; `js/runtime/_reactive.js` (transitional) puts Solid's API on top of it.
- **Apps compiled by nsh** (`"compiler": "nsh"`, `native-pipeline.md`) import the native modules `nsh` and `nsh:rt`
  instead: the renderer pushes ops straight into the host's queue, with no JS runtime in the bundle.
- **Text is laid out once**: `TextEngine` (defined in core) runs `prepare` on the engine thread (Apple: CTFramesetter) → `measure` at various widths →
  `layout` at the final size after layout (Apple: CTFrame). `MountItem::Text` hands the immutable result to the main thread, which only draws it
  (`CTFrameDraw` in `TextView`'s `drawRect`); the main thread never lays out text. Android's counterpart is `StaticLayout` (like RN's PreparedLayout).
  Implementations must not touch main-thread-only UI objects.
- Measure cache: one content-addressed LRU per engine (`MeasureCache`, two generations of hash maps, ≤1024 entries), keyed by a hash of the
  layout-relevant content (`AttributedText::layout_hash`, colors excluded) + the width constraint; nodes also cache `Prepared` and the final layout.
  A color-only change does not trigger layout, only a new final layout.
- One `Surface` (Apple) / `NshView` (Android) is one engine (one QuickJS runtime), with no global singletons, so nsh can be embedded in
  existing apps with several instances. `AppView` (Apple) wraps a surface for embedding and owns its dev session; a standalone app is
  just "one AppView filling the window".
- **Android**: the engine thread typesets with `StaticLayout` through JNI; each commit is one `byte[]` plus an object
  table handed to `Mounter.java` on the UI thread, and animation items stay in Rust (the flow, the units and the JNI
  rules: `crates/nsh-platform-android/src/encode.rs`).
- **Networking**: `fetch` (and `Headers`, `Request`, `Response`, `AbortController`, `URL`, `URLSearchParams`, `TextEncoder`/`Decoder`)
  is defined by `crates/nsh-js/src/web.js`, precompiled to bytecode at build time and loaded on first use of any of those globals
  (zero startup cost for apps that do not use them). Requests go to the platform stack through `nsh_engine::Network`
  (NSURLSession, HttpURLConnection; ureq for `nsh trace`), streaming both ways: the stack reports the response head,
  then body chunks as they arrive (`FetchEvent`), and JS can pause reading (back pressure); a streamed request body is
  sent in chunks the stack asks for one at a time (`FetchEvent::BodyWanted`, chunked transfer encoding).

## Crates

Dependency direction: `nsh-core` ← `nsh-js` ← `nsh-engine` (also ← `nsh-package`) ← `nsh-platform-*` ← `nsh-ios-host` / `nsh-ios-app`; `nsh-cli` depends on `nsh-bundler`, `nsh-js` and `nsh-engine`; `nsh-ui-mobile` (the mobile components, `components.md`) depends on `nsh-core` only. `nsh-core` must not depend on any platform or JS crate.

The CoreText and StaticLayout text engines are documented in `crates/nsh-platform-ios/src/text.rs` and
`crates/nsh-platform-android/src/text.rs`, the measure cache in `crates/nsh-core/src/text.rs` (`MeasureCache`).
