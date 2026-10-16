//! The Rust gallery (`examples/gallery-rs`): the app's TSX mounts one
//! component of this crate, and everything it shows is written in Rust.
//! The app imports the set as `gallery/native` (`nsh.json` `"rust"`).

use nsh_core::ui::{Registry, prelude::*};

/// The UI set of the gallery: what `App.tsx` imports from `gallery/native`.
pub fn registry() -> Registry {
    let mut registry = Registry::ui_set("gallery/native");
    registry.register::<Gallery>();
    registry
}

/// The gallery.
#[derive(Debug, Default)]
pub struct Gallery {
    taps: u32,
}

impl Component for Gallery {
    const NAME: &'static str = "gallery.gallery";
    type Props = ();
    type Event = NoEvent;

    fn render(&self, _: &(), cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        let theme = cx.theme();
        view()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_4()
            .bg(theme.background)
            .child(
                text("Rust gallery")
                    .text_size(px(28.0))
                    .font_weight(FontWeight::BOLD),
            )
            .child(
                text(format!("Tapped {} times", self.taps))
                    .text_color(theme.muted_foreground)
                    .on_press(cx.listener(|this, _, cx| {
                        this.taps += 1;
                        cx.notify();
                    })),
            )
    }
}
