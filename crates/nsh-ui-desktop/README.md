# nsh-ui-desktop

The components of `nsh/ui/desktop`, written in Rust: NSH's design language for a pointer and a keyboard. The same
theme colors, radii, font size, motion (`nsh_core::ui::motion`) and dimming (`DISABLED_OPACITY`) as `nsh/ui/mobile`,
at a desktop's density; one implementation draws the same on macOS, Windows, Linux, GPUI and the web.

Components are written exactly as the mobile set's (`crates/nsh-ui-mobile/README.md`: props, events, bindings,
regions, slots, state, tasks, modules); this set adds what a desktop control is:

- **Density from `metrics.rs`**: a control's `size: Prop<ControlSize>` (`sm`, `md` by default, `lg`) gives its height
  (24, 32, 40 points), its padding, its text size (the theme's font size, smaller for the denser sizes) and its corner
  radius (the theme's, scaled from the mobile set's 44-point controls by height, so the shape is the mobile one). Read
  them in bindings: `.h(move || px(size.get().height()))`, `.rounded(move || px(size.get().radius()))`.
- **The pointer**: `.hover(|s| ..)` for what is under it (a background faded to `HOVER_OPACITY`, or the theme's
  `accent` for controls without one), `.active(|s| ..)` while pressed (the theme's `*_active` colors, as on mobile).
- **The keyboard**: `.focus(|s| s.outline(RING_WIDTH).outline_color(move || theme().ring).outline_offset(RING_OFFSET))`.
  Pressables take focus from Tab only, never from a click, so a ring on `focus` shows exactly for keyboard focus;
  Return and Space press the focused control natively.
- **Shared, not copied**: what a control does (a value the app may control, a number in steps, a press that repeats)
  is `nsh_core::ui`'s (`controllable`, `NumberRange`, `press_repeat`), the same as the mobile set uses. A component
  never imports from `nsh-ui-mobile`; what both need moves to the core.
- **Floating surfaces** (`Popover`) are a transparent modal over the window: a backdrop that closes them on a click
  (`focusable(false)`, so Tab goes straight to the content), Escape through the modal's `on_request_close`, the
  trigger measured (`ElementRef::measure`) and the surface placed with the core's `place` (flipped and kept in the
  window), hidden until it is placed, faded out with `motion::EXIT` while the core's `presence` keeps it shown. The
  platforms give the keyboard focus back to the trigger when it closes.
- **One file per component** (`src/button.rs`), registered in `registry()` (`src/lib.rs`); `the_exports_are_pinned`
  lists the exports. Exports may share a name with the mobile set's (`Button`): apps import the set they draw with,
  or both under different names.
