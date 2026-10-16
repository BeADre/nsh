//! Web and video: WKWebView / WebView, AVPlayer / MediaPlayer (as the TSX gallery's `demos/Media.tsx`).

use nsh_core::ui::prelude::*;

use crate::ui::{colors, page, section};

/// The demo's page.
#[derive(Debug, Default)]
pub(crate) struct Media;

impl Component for Media {
    const NAME: &'static str = "gallery.media";
    type Props = ();
    type Event = NoEvent;

    fn render(&self, _: &(), _cx: &mut RenderCx<'_, Self>) -> impl IntoElement {
        view().flex_1().child(page([section(
            "Web and video",
            [text("Not ported yet.")
                .text_color(colors::muted())
                .into_element()],
        )
        .into_element()]))
    }
}
