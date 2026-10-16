//! Typed styles: What Tailwind classes do, as typed style methods: variants, grid, shadows, gradients, motion (as the TSX gallery's `demos/Tailwind.tsx`).

use nsh_core::ui::prelude::*;

use crate::ui::{colors, page, section};

/// The demo's page.
#[derive(Debug, Default)]
pub(crate) struct TypedStyles;

impl Component for TypedStyles {
    const NAME: &'static str = "gallery.typed_styles";
    type Props = ();
    type Event = NoEvent;

    fn render(&self, _: &(), _cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        view().flex_1().child(page([section(
            "Typed styles",
            [text("Not ported yet.")
                .text_color(colors::muted())
                .into_element()],
        )
        .into_element()]))
    }
}
