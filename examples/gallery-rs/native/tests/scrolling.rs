//! The Scrolling demo on a headless engine: its sections render, drags
//! are logged, the zoom buttons, the feed and the keyboard taps choice
//! change what shows.

mod common;

use crossbeam_channel::Receiver;
use nsh_core::{EventKind, EventPayload, MountBatch};
use nsh_engine::Engine;

/// Presses the text `label` itself (a choice: a text with its own press
/// handler), and waits for what it changes.
fn press_text(
    engine: &Engine,
    rx: &Receiver<MountBatch>,
    screen: &mut common::Screen,
    label: &str,
) {
    let text = screen
        .find(label)
        .unwrap_or_else(|| panic!("no {label}: {:?}", screen.texts));
    engine
        .control()
        .dispatch_event(text, EventKind::Press, EventPayload::None);
    common::drain(rx, screen);
}

fn shows(screen: &common::Screen, label: &str) -> bool {
    screen.find(label).is_some()
}

#[test]
fn scrolling_sections_react() {
    let (engine, rx, mut screen) = common::open("Scrolling");
    for label in [
        "PAGING",
        "Page 1",
        "SNAPPING (SNAPTOINTERVAL)",
        "Card 1",
        "center",
        "ZOOM (PINCH, TAP A TILE)",
        "Tile 1",
        "Zoom 1.00×",
        "EVENTS AND BEHAVIOR",
        "Line 1",
        "Scroll the box",
        "No drags yet",
        "Bounces",
        "PREPENDING KEEPS THE POSITION",
        "Post 1",
        "Pressed 0 times",
        "A tap outside the input only hides the keyboard; the button does not get it.",
    ] {
        assert!(shows(&screen, label), "{label}: {:?}", screen.texts);
    }

    // A drag on the events' box is logged.
    let line = screen.find("Line 1").expect("a line");
    let scroll = screen.parents[&screen.parents[&line]];
    engine.control().dispatch_event(
        scroll,
        EventKind::ScrollBeginDrag,
        EventPayload::Scroll { x: 0.0, y: 12.0 },
    );
    common::drain(&rx, &mut screen);
    assert!(shows(&screen, "begin drag @ 12"), "{:?}", screen.texts);

    common::press(&engine, &rx, &mut screen, "2×");
    assert!(shows(&screen, "Zoom 2.00×"), "{:?}", screen.texts);

    common::press(&engine, &rx, &mut screen, "Add 3 posts on top");
    assert!(shows(&screen, "New 1"), "{:?}", screen.texts);

    press_text(&engine, &rx, &mut screen, "handled");
    assert!(
        shows(
            &screen,
            "The button gets the tap and the keyboard stays; a tap on empty space hides it."
        ),
        "{:?}",
        screen.texts
    );

    common::press(&engine, &rx, &mut screen, "Pressed 0 times");
    assert!(shows(&screen, "Pressed 1 times"), "{:?}", screen.texts);
}
