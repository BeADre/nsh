# Classes from Rust (design proposal)

Status: proposed.

An app's Rust crate (`nsh.json` `"rust"`) exports components and modules (`#[module]`: functions). This proposes
classes: a Rust type that TypeScript uses as a class, with a constructor, methods, getters and setters, statics and
instances that keep their state in Rust.

```rust
/// A tally kept in Rust.
#[class]
pub struct Tally {
    count: i64,
    log: Vec<String>,
}

#[class]
impl Tally {
    /// Starts at `start` (0 when left out).
    pub fn new(start: Option<i64>) -> Self {
        Tally { count: start.unwrap_or(0), log: Vec::new() }
    }

    /// Adds `by`; the new count.
    pub fn add(&mut self, by: i64) -> i64 {
        self.count += by;
        self.log.push(format!("+{by}"));
        self.count
    }

    /// The count.
    #[getter]
    pub fn count(&self) -> i64 {
        self.count
    }

    #[setter]
    pub fn set_count(&mut self, count: i64) {
        self.count = count;
    }

    /// A tally from `"12"`; throws on anything else.
    pub fn parse(text: String) -> Result<Tally, String> {
        Ok(Tally::new(Some(text.trim().parse().map_err(|_| format!("not a number: {text}"))?)))
    }

    /// Saves the log; resolves when it is written.
    pub async fn save(this: Instance<Tally>, path: String) -> Result<(), String> {
        let lines = this.with(|t| t.log.join("\n"));
        blocking(move || std::fs::write(path, lines)).await.map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())
    }
}

#[native_module("app/native")]
fn native(registry: &mut Registry) {
    registry.class::<Tally>();
}
```

```ts
// .nsh/types/app/native.d.ts, generated
/** A tally kept in Rust. */
export declare class Tally {
  /** Starts at `start` (0 when left out). */
  constructor(start?: number);
  /** Adds `by`; the new count. */
  add(by: number): number;
  /** The count. */
  get count(): number;
  set count(count: number);
  /** A tally from `"12"`; throws on anything else. */
  static parse(text: string): Tally;
  /** Saves the log; resolves when it is written. */
  save(path: string): Promise<void>;
  /** Frees the Rust value now (else when the object is collected); the object throws after. */
  dispose(): void;
}
```

```tsx
import { Tally } from "app/native";
const tally = new Tally(3);
tally.add(2);            // 5, synchronously
tally.count = 10;
await tally.save("/tmp/log");
```

## Why synchronous

Module functions are ops: JS queues them and the core runs them after the turn, so they always return a promise.
That is fine for "do something" (show an alert) but not for something that looks like a class: a getter cannot be a
promise, and `tally.add(2) + 1` should be a number. JS and Rust components already run on one thread (the engine's),
so a class's methods can run directly inside the JS call, as the web classes nsh-js implements in Rust (`Blob`,
`Headers`, `URL`) do.

## The model

- **Instances live in Rust, owned by their JS object.** A JS object of the class holds the Rust value as its opaque
  (`Rc<RefCell<T>>`, the web classes' slot, `nsh-js` `web/mod.rs`); the object being collected drops it.
  `dispose()` drops it now (a value holding a file or a large buffer); any use after throws. `T: 'static`, not
  `Send`: it never leaves the engine thread.
- **Synchronous members run in the JS call.** `&self` / `&mut self` methods, getters and setters, statics and the
  constructor run inside the JS turn, outside the core: they compute and keep state, and cannot use the UI's free
  functions (signals, `dialog`, `theme()`): the reactive graph is not entered there, and calling one panics with a
  message saying so (as on any thread without it). A method borrows its instance for its call; a re-entrant call
  (a method reaching the same instance again) throws instead of aliasing, as the web classes do.
- **`async` members are core tasks.** An `async fn` method gets `this: Instance<Self>` (a handle: `with` /
  `with_mut` borrow the value between awaits) and returns a promise; it runs as a task of the core, where the free
  functions work (`dialog`, `timer`, `blocking`, `background`, the clipboard), as an `async` module function does.
  The call is an op like a module function's (`Command::Native { name: "Tally#save", args: { c, this, a } }`); the
  instance reaches the core through a table both sides share (`Instances`, made by the engine and handed to
  `nsh-js` and the core), keyed by an id the JS object carries. The task keeps the instance alive until it ends.
- **Errors.** `Result<T, E: Display>` throws (`Error` with the message) or rejects; a wrong argument throws a
  `TypeError` naming the parameter; a panic in Rust is caught and thrown as an `Error` (the engine stays up).

## What crosses

- **Arguments and results** are the module functions' types (`JsValue`: numbers, `bool`, strings, `Option`, `Vec`,
  `JsonValue`, `#[derive(JsObject)]` structs, `#[derive(PropEnum)]` enums), so classes and modules share one set of
  conversions and TypeScript types; a trailing `Option` parameter is optional.
- **Instances of classes**, by reference: a parameter `&Other` / `&mut Other` borrows another instance for the call
  (TS: `other: Other`), `Instance<Other>` keeps a handle (to store one instance in another), and a result `Other`
  or `Instance<Other>` is a new JS object (`Self` from `new` and statics). An instance of another app's or set's
  class is refused with a `TypeError`.
- **Not in the first version:** JS functions as arguments (callbacks: they need JS handles kept from Rust), events
  emitted by an instance (`EventTarget`), iterators, and subclassing in TS (the web classes support `new.target`;
  app classes can follow once something needs it).

## The pieces

- **`nsh-macros`:** `#[class]` on the struct (a marker; the type's name is the JS name) and on its `impl`: `new`
  is the constructor (a class without one cannot be made from JS, only returned); `&self` / `&mut self` methods,
  `#[getter]` / `#[setter]` (one name: `count` and `set_count` are the property `count`), functions without `self`
  are statics, `async fn` with `this: Instance<Self>` are async methods. Doc comments become the declarations'.
  It implements `ui::Class` (name, doc, members, each member a decoder of its arguments and a call), as `#[module]`
  implements `Module`.
- **`nsh-core`:** `ui::Class`, `ClassSpec` (for the JS face and the declarations), `Instance<T>`, the `Instances`
  table, `Registry::class::<C>()`; the core runs async members like async module functions (`call_function`).
- **`nsh-js`:** makes each registered class a QuickJS class when a bundle imports the set (as it makes modules'
  objects in `components.rs`): its constructor, prototype methods, accessors and statics call the spec's members
  with the converted arguments; async members push the op and return the promise; the finalizer drops the slot.
- **Declarations:** `nsh_core::ui::declarations` writes `export declare class` for each, with the types the members
  use, next to the set's components and modules; the declarations test checks that what exists is declared.
- **The CLI** needs no change: classes are part of the app's set like its modules.

## Costs

A synchronous call converts its arguments from QuickJS values to `JsonValue` and decodes them, as a module call
does, but without the op, the commit and the promise: a few microseconds for small arguments. If a class turns out
to be called in hot loops, direct converters between QuickJS values and the Rust types are the next step; the
benchmark comes with the implementation (`docs/dev/performance.md`).

## Decisions to make

1. Synchronous members run in the JS call and cannot use the UI's free functions; `async` members can. (The
   alternative, every member a promise, keeps one model but does not look like a class.)
2. `dispose()` besides collection, for values that hold resources.
3. The first version's types: the module functions' plus instances by reference; callbacks and events later.
