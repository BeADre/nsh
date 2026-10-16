//! The Physics demo on a headless engine: releases are counted, a long
//! press resets the count.

use crossbeam_channel::Receiver;
use nsh_core::{EventKind, EventPayload, MountBatch};
use nsh_engine::Engine;

use crate::common::shows;

/// Sends `kind` with `payload` to the parent of the text `label`.
fn dispatch(
    engine: &Engine,
    rx: &Receiver<MountBatch>,
    screen: &mut crate::common::Screen,
    label: &str,
    kind: EventKind,
    payload: EventPayload,
) {
    let text = screen.find(label).unwrap_or_else(|| panic!("no {label}"));
    let target = screen.parents[&text];
    engine.control().dispatch_event(target, kind, payload);
    crate::common::drain(rx, screen);
}

#[test]
fn physics_counts_releases() {
    let (engine, rx, mut screen) = crate::common::open("Physics");
    assert!(shows(
        &screen,
        "Drag the card, tap it to pulse, long-press to reset"
    ));
    assert!(shows(&screen, "Released 0 times"), "{:?}", screen.texts);

    let release = EventPayload::Pan {
        translation_x: 80.0,
        translation_y: 20.0,
        velocity_x: 300.0,
        velocity_y: -100.0,
        value_x: Some(80.0),
        value_y: Some(20.0),
    };
    dispatch(
        &engine,
        &rx,
        &mut screen,
        "Drag me",
        EventKind::PanEnd,
        release.clone(),
    );
    dispatch(
        &engine,
        &rx,
        &mut screen,
        "Drag me",
        EventKind::PanEnd,
        release,
    );
    assert!(shows(&screen, "Released 2 times"), "{:?}", screen.texts);

    // A tap pulses (natively; the count stays).
    crate::common::press(&engine, &rx, &mut screen, "Drag me");
    assert!(shows(&screen, "Released 2 times"), "{:?}", screen.texts);

    dispatch(
        &engine,
        &rx,
        &mut screen,
        "Drag me",
        EventKind::LongPress,
        EventPayload::None,
    );
    assert!(shows(&screen, "Released 0 times"), "{:?}", screen.texts);
}
