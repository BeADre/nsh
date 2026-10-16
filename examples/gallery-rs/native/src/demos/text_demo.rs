//! Text: Fonts, spans, inline views and images, decorations, truncation, shadows (as the TSX gallery's `demos/TextDemo.tsx`).

use nsh_core::ui::prelude::*;

use crate::ui::{colors, page, section};

/// The demo's page.
#[derive(Debug, Default)]
pub(crate) struct TextDemo;

impl Component for TextDemo {
    const NAME: &'static str = "gallery.text_demo";
    type Props = ();
    type Event = NoEvent;

    fn render(&self, _: &(), _cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        view().flex_1().child(page([section(
            "Text",
            [text("Not ported yet.")
                .text_color(colors::muted())
                .into_element()],
        )
        .into_element()]))
    }
}
