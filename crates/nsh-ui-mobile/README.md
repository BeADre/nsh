# nsh-ui-mobile

The components and modules of `nsh/ui/mobile`, those phones and tablets have: the status bar (`StatusBar`), sheets
(`Sheet`), action sheets (`ActionSheet`) and the software keyboard (`Keyboard`, `keyboard`, `KeyboardAvoidingView`),
written in Rust. One implementation draws the same on every platform.

The components every form factor shares (buttons, cards, lists, text fields, safe areas, alerts, navigation) are
`nsh/ui`'s, in `nsh-ui`. Components here are written the same way: see `crates/nsh-ui/README.md`. Each is registered
in `registry()` (`src/lib.rs`), and `the_exports_are_pinned` lists the exports.

How the core runs components: `docs/dev/components.md`.
