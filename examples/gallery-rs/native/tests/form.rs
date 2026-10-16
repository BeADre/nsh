//! The Form demo on a headless engine: its inputs render, submitting and
//! the dismiss modes change what it shows, the message bar pushes.

mod common;

use crossbeam_channel::Receiver;
use nsh_core::{EventKind, EventPayload, MountBatch};
use nsh_engine::Engine;

/// Presses the text `label` itself (texts that take presses: the modes).
fn press_text(
    engine: &Engine,
    rx: &Receiver<MountBatch>,
    screen: &mut common::Screen,
    label: &str,
) {
    let id = screen
        .find(label)
        .unwrap_or_else(|| panic!("no {label}: {:?}", screen.texts));
    engine
        .control()
        .dispatch_event(id, EventKind::Press, EventPayload::None);
    common::drain(rx, screen);
}

fn shows(screen: &common::Screen, label: &str) -> bool {
    screen.find(label).is_some()
}

#[test]
fn form_renders_and_submits() {
    let (engine, rx, mut screen) = common::open("Form");
    for label in [
        "Keyboard 0 pt",
        "ACCOUNT",
        "E-mail",
        "Code (6 digits)",
        "Selection 0–0: \"\"",
        "Keys: –",
        "Notes (multiline, grows, 280 max)",
        "0/280",
        "caretHidden",
    ] {
        assert!(shows(&screen, label), "{label}: {:?}", screen.texts);
    }
    common::press(&engine, &rx, &mut screen, "Submit");
    assert!(shows(&screen, "Nobody <no e-mail>"), "{:?}", screen.texts);
    // The modes are texts taking presses; one changes the scroll view's
    // mode and renders again (the keyboard line stays).
    press_text(&engine, &rx, &mut screen, "interactive");
    assert!(shows(&screen, "Keyboard 0 pt"));
    common::press(&engine, &rx, &mut screen, "Inputs in a sheet");
    assert!(shows(&screen, "A sheet"), "{:?}", screen.texts);
    assert!(shows(&screen, "Seventh"), "{:?}", screen.texts);
}

#[test]
fn form_pushes_the_message_bar() {
    let (engine, rx, mut screen) = common::open("Form");
    common::press(&engine, &rx, &mut screen, "A message bar at the bottom");
    assert!(
        screen.titles.iter().any(|t| t.1 == "Message bar"),
        "{:?}",
        screen.titles
    );
    assert!(shows(&screen, "Send"), "{:?}", screen.texts);
}
