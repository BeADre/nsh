//! The Photos demo on a headless engine: its rows render, more load at the
//! end, a pull refreshes.

mod common;

use std::time::Duration;

use crossbeam_channel::Receiver;
use nsh_core::{EventKind, EventPayload, MountBatch};
use nsh_engine::Engine;

/// Sends `kind` to every view above the text `label` (the one listening
/// handles it), and waits for what it changes.
fn dispatch_above(
    engine: &Engine,
    rx: &Receiver<MountBatch>,
    screen: &mut common::Screen,
    label: &str,
    kind: EventKind,
) {
    let mut node = screen
        .find(label)
        .unwrap_or_else(|| panic!("no {label}: {:?}", screen.texts));
    while let Some(&parent) = screen.parents.get(&node) {
        engine
            .control()
            .dispatch_event(parent, kind, EventPayload::None);
        node = parent;
    }
    common::drain(rx, screen);
}

fn shows(screen: &common::Screen, label: &str) -> bool {
    screen.find(label).is_some()
}

#[test]
fn photos_load_more_and_refresh() {
    let (engine, rx, mut screen) = common::open("Photos");
    for label in [
        "1000 photos",
        "Photo #1",
        "by Linus",
        "Photo #2",
        "by Grace",
    ] {
        assert!(shows(&screen, label), "{label}: {:?}", screen.texts);
    }

    // The end of the list loads 200 more.
    dispatch_above(&engine, &rx, &mut screen, "Photo #1", EventKind::EndReached);
    assert!(shows(&screen, "1200 photos"), "{:?}", screen.texts);

    // A pull refreshes: back to the first 1000 after a moment.
    dispatch_above(&engine, &rx, &mut screen, "Photo #1", EventKind::Refresh);
    std::thread::sleep(Duration::from_millis(900));
    common::drain(&rx, &mut screen);
    assert!(shows(&screen, "1000 photos"), "{:?}", screen.texts);
}
