//! The Animations demo on a headless engine: its sections render, the
//! compositions report starting and stopping, the interpolation's chips
//! and the layout's button follow their state.

mod common;

use crossbeam_channel::Receiver;
use nsh_core::{EventKind, EventPayload, MountBatch};
use nsh_engine::Engine;

/// Presses the text `label` itself (a chip: the text takes the press).
fn press_chip(
    engine: &Engine,
    rx: &Receiver<MountBatch>,
    screen: &mut common::Screen,
    label: &str,
) {
    let chip = screen.find(label).unwrap_or_else(|| panic!("no {label}"));
    engine
        .control()
        .dispatch_event(chip, EventKind::Press, EventPayload::None);
    common::drain(rx, screen);
}

fn has(screen: &common::Screen, text: &str) -> bool {
    screen.texts.iter().any(|t| t.1 == text)
}

#[test]
fn animations_render_and_react() {
    let (engine, rx, mut screen) = common::open("Animations");
    for text in [
        "EASING CURVES",
        "SEQUENCE, PARALLEL, STAGGER, LOOP",
        "SPRINGS",
        "INTERPOLATION",
        "DIFFCLAMP: A HEADER THAT HIDES ON SCROLL",
        "LAYOUT ANIMATION",
        "bounce out",
        "wobbly",
        "idle",
        "Scroll up to hide me",
        "Row 30",
        "t = 0 ●",
    ] {
        assert!(has(&screen, text), "{text}: {:?}", screen.texts);
    }

    press_chip(&engine, &rx, &mut screen, "Stagger");
    assert!(has(&screen, "stagger…"), "{:?}", screen.texts);
    press_chip(&engine, &rx, &mut screen, "Loop");
    assert!(has(&screen, "loop…"), "{:?}", screen.texts);
    // The last one going on reports.
    press_chip(&engine, &rx, &mut screen, "Stop");
    assert!(has(&screen, "loop: stopped"), "{:?}", screen.texts);
    press_chip(&engine, &rx, &mut screen, "Sequence");
    assert!(has(&screen, "sequence…"), "{:?}", screen.texts);
    press_chip(&engine, &rx, &mut screen, "Reset");
    assert!(has(&screen, "sequence: stopped"), "{:?}", screen.texts);

    press_chip(&engine, &rx, &mut screen, "t = 2");
    assert!(
        has(&screen, "t = 2 ●") && has(&screen, "t = 0"),
        "{:?}",
        screen.texts
    );

    common::press(&engine, &rx, &mut screen, "Column");
    assert!(has(&screen, "Row"), "{:?}", screen.texts);
    common::press(&engine, &rx, &mut screen, "Row");
    assert!(has(&screen, "Column"), "{:?}", screen.texts);
}
