//! `Kbd`: a keyboard shortcut, as this platform writes it.

use nsh_core::{keymap::Keystroke, ui::prelude::*};

use crate::metrics::{label_font_size, radius_for};

/// How tall the chip is, in points.
const HEIGHT: f32 = 20.0;

/// A keyboard shortcut in a key-shaped chip, written as menus write it on
/// this platform: `"mod+k"` reads `⌘K` on macOS (and in a browser on a
/// Mac), `Ctrl+K` elsewhere.
#[derive(Debug, Clone, PartialEq, Props)]
pub struct KbdProps {
    /// The keystroke, as `nsh/keymap` writes it: `"mod+k"`,
    /// `"shift+mod+p"`, `"escape"` (`mod` is Command on Apple platforms,
    /// Control elsewhere). Text that is not a keystroke is shown as it is.
    pub keys: Prop<SharedString>,
}

/// What `keys` reads as here: its label when it is a keystroke under
/// either platform's conventions (`ctrl+mod+k` is one only where `mod` is
/// Command, and `key_label` takes only keystrokes), the text otherwise.
fn label(keys: &str) -> String {
    let valid =
        Keystroke::parse_with(keys, true).is_ok() && Keystroke::parse_with(keys, false).is_ok();
    if valid {
        key_label(keys)
    } else {
        keys.to_owned()
    }
}

/// A keyboard shortcut in a chip ([`KbdProps`]).
#[component]
pub fn Kbd(props: KbdProps) -> impl IntoElement {
    let shown = computed(move || label(&props.keys.get()));
    view()
        .flex_row()
        .items_center()
        .justify_center()
        .h(px(HEIGHT))
        .min_w(px(HEIGHT))
        .px(px(5.0))
        .rounded(move || px(radius_for(HEIGHT)))
        .bg(move || theme().muted)
        .border_1()
        .border_color(move || theme().border)
        .child(
            text(move || shown.get())
                .number_of_lines(1)
                .text_size(move || px(label_font_size()))
                .font_weight(FontWeight::MEDIUM)
                .text_color(move || theme().muted_foreground),
        )
}
