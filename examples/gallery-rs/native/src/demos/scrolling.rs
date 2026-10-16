//! Scrolling: Paging, snapping, zoom, scroll events, prepending, keyboard taps (as the TSX gallery's `demos/Scrolling.tsx`).

use nsh_core::ui::prelude::*;

use crate::ui::{colors, page, section};

/// The demo's page.
#[derive(Debug, Default)]
pub(crate) struct Scrolling;

impl Component for Scrolling {
    const NAME: &'static str = "gallery.scrolling";
    type Props = ();
    type Event = NoEvent;

    fn render(&self, _: &(), _cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        view().flex_1().child(page([section(
            "Scrolling",
            [text("Not ported yet.")
                .text_color(colors::muted())
                .into_element()],
        )
        .into_element()]))
    }
}
