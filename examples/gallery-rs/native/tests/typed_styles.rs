//! The Typed styles demo: the Tailwind demo's cards in typed styles, its
//! buttons' state and the canvases standing in for gradients.

mod common;

use std::time::Duration;

use crossbeam_channel::Receiver;
use nsh_core::{MountBatch, MountItem, ViewKind};

fn shows(screen: &common::Screen, text: &str) -> bool {
    screen.texts.iter().any(|t| t.1 == text)
}

/// The canvases `rx`'s batches create, applying them to `screen`.
fn canvases(rx: &Receiver<MountBatch>, screen: &mut common::Screen) -> usize {
    let mut created = 0;
    while let Ok(batch) = rx.recv_timeout(Duration::from_millis(300)) {
        screen.apply(&batch);
        created += batch
            .items
            .iter()
            .filter(|i| {
                matches!(
                    i,
                    MountItem::Create {
                        kind: ViewKind::Canvas,
                        ..
                    }
                )
            })
            .count();
    }
    created
}

#[test]
fn typed_styles_shows_every_card() {
    let (_engine, _rx, screen) = common::open("Typed styles");
    for text in [
        "BUTTONS: ACTIVE, HOVER, TRANSITION, DISABLED",
        "CARDS: SHADOW, RING, GROUP, GRADIENT",
        "FLEX, GRID, BREAKPOINTS",
        "PALETTE",
        "SHADOWS, RINGS, OUTLINES",
        "GRADIENTS",
        "TYPOGRAPHY",
        "FILTERS, BACKDROP BLUR, BLEND MODES",
        "ANIMATIONS AND TRANSFORMS",
        "FIRST:, EVEN:, LAST:",
        "INPUTS",
        "Group press",
        "Gradient card",
        "col-span-2",
        "emerald",
        "2xl",
        "Display",
        "blur-sm",
        "bounce",
        "Archive",
    ] {
        assert!(shows(&screen, text), "{text}: {:?}", screen.texts);
    }
}

#[test]
fn typed_styles_buttons_keep_their_state() {
    let (engine, rx, mut screen) = common::open("Typed styles");
    common::press(&engine, &rx, &mut screen, "Pressed 0×");
    common::press(&engine, &rx, &mut screen, "Pressed 1×");
    assert!(shows(&screen, "Pressed 2×"), "{:?}", screen.texts);
    common::press(&engine, &rx, &mut screen, "Reset");
    assert!(shows(&screen, "Pressed 0×"), "{:?}", screen.texts);

    common::press(&engine, &rx, &mut screen, "Enabled");
    assert!(shows(&screen, "Disabled"), "{:?}", screen.texts);
    common::press(&engine, &rx, &mut screen, "Enable again");
    assert!(shows(&screen, "Enabled"), "{:?}", screen.texts);
}

#[test]
fn typed_styles_draws_gradients_on_canvases() {
    let (engine, rx) = common::start();
    let mut screen = common::Screen::default();
    common::drain(&rx, &mut screen);
    let text = screen.find("Typed styles").expect("the demo in the index");
    engine.control().dispatch_event(
        screen.parents[&text],
        nsh_core::EventKind::Press,
        nsh_core::EventPayload::None,
    );
    // The four gradients' canvases; the gradient card's comes once the
    // card knows its size.
    assert_eq!(canvases(&rx, &mut screen), 4);
    let title = screen.find("Gradient card").expect("the gradient card");
    engine.control().dispatch_event(
        screen.parents[&title],
        nsh_core::EventKind::Layout,
        nsh_core::EventPayload::Layout {
            x: 0.0,
            y: 0.0,
            width: 326.0,
            height: 100.0,
        },
    );
    assert_eq!(canvases(&rx, &mut screen), 1);
}
