//! The Animations demo on a headless engine: its sections render, the
//! compositions report starting and stopping, the interpolation's chips
//! and the layout's button follow their state.

use crate::common::{press_text, shows};

#[test]
fn animations_render_and_react() {
    let (engine, rx, mut screen) = crate::common::open("Animations");
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
        assert!(shows(&screen, text), "{text}: {:?}", screen.texts);
    }

    press_text(&engine, &rx, &mut screen, "Stagger");
    assert!(shows(&screen, "stagger…"), "{:?}", screen.texts);
    press_text(&engine, &rx, &mut screen, "Loop");
    assert!(shows(&screen, "loop…"), "{:?}", screen.texts);
    // The last one going on reports.
    press_text(&engine, &rx, &mut screen, "Stop");
    assert!(shows(&screen, "loop: stopped"), "{:?}", screen.texts);
    press_text(&engine, &rx, &mut screen, "Sequence");
    assert!(shows(&screen, "sequence…"), "{:?}", screen.texts);
    press_text(&engine, &rx, &mut screen, "Reset");
    assert!(shows(&screen, "sequence: stopped"), "{:?}", screen.texts);

    press_text(&engine, &rx, &mut screen, "t = 2");
    assert!(
        shows(&screen, "t = 2 ●") && shows(&screen, "t = 0"),
        "{:?}",
        screen.texts
    );

    crate::common::press(&engine, &rx, &mut screen, "Column");
    assert!(shows(&screen, "Row"), "{:?}", screen.texts);
    crate::common::press(&engine, &rx, &mut screen, "Row");
    assert!(shows(&screen, "Column"), "{:?}", screen.texts);
}
