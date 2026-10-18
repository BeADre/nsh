# Several Rust crates in one app (design proposal)

Status: proposed.

Today an app has at most one Rust crate (`nsh.json` `"rust": "<crate dir>"`, a string), with one
`#[native_module("<specifier>")]` entry, so everything Rust it offers is under one import specifier
(`docs/dev/tooling.md`, "Apps with Rust components of their own"). Splitting an app's native code (charts, storage, a
shared crate of company components) means one crate that depends on the others and registers them all under its
specifier.

## Proposal

- `"rust"` takes a crate or a list of them: `"rust": ["native/charts", "native/storage"]`. Each is a library crate
  with its own `#[native_module("<specifier>")]` (one per crate, as now), so each is its own import:
  `import { Chart } from "app/charts"`, `import { Store } from "app/storage"`.
- `rust_app.rs` writes one generated workspace with all of them: the command line and every host and runner depend on
  each crate and pass all their registries (`main_with(ui_sets)`, `set_ui_sets`), after `nsh/ui/mobile`.
- Checked when the workspace is written: two crates with the same package name, or two entries with the same
  specifier (`Registry::merged` already refuses two sets exporting the same component or module name within one set,
  and different sets may share names), fail with what to change.
- Each specifier gets its declarations (`.nsh/types/<specifier>.d.ts`), its assets (`asset:/@<package>/...`, already
  per package) and is watched by `nsh dev` (a change to any crate restarts the app's command line, as now).
- Crates may depend on each other (a shared crate of components that another crate's components use) through plain
  Cargo path dependencies; only the ones listed in `"rust"` export to TS.

## Why a list of crates

The alternative, several `#[native_module]` entries in one crate, needs a collection of entries the generated code
can find (today it finds one entry by a fixed name), and keeps everything in one compilation unit. A crate per
specifier is Rust's own unit of reuse: a crate of shared components can be listed by several apps, versioned and
tested on its own.
