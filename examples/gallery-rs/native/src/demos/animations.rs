//! Animations: Easing, sequence / parallel / stagger / loop, springs, interpolation, diffClamp (as the TSX gallery's `demos/Animations.tsx`).

use nsh_core::ui::prelude::*;

use crate::ui::{colors, page, section};

/// The demo's page.
#[derive(Debug, Default)]
pub(crate) struct Animations;

impl Component for Animations {
    const NAME: &'static str = "gallery.animations";
    type Props = ();
    type Event = NoEvent;

    fn render(&self, _: &(), _cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        view().flex_1().child(page([section(
            "Animations",
            [text("Not ported yet.")
                .text_color(colors::muted())
                .into_element()],
        )
        .into_element()]))
    }
}
