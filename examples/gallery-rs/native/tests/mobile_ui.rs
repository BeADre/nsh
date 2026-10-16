//! The Mobile UI demo on a headless engine: the header's components and
//! the first rows render, the header's buttons and steppers change its
//! state.

mod common;

fn shows(screen: &common::Screen, label: &str) -> bool {
    screen.find(label).is_some()
}

#[test]
fn mobile_ui_renders_and_answers_presses() {
    let (engine, rx, mut screen) = common::open("Mobile UI");
    for label in [
        "Name (controlled; × clears and focuses)",
        "Hello, Ada",
        "SelectList: 500 rows of Rust data, chosen: none",
        "City 1",
        "Group: each child is a row",
        "Notifications",
        "Controlled: 2",
        "Ask the stepper, then reset it",
        "Inset rows",
        // The first rows below the header.
        "SYM000",
        "-2.50%",
    ] {
        assert!(shows(&screen, label), "{label}: {:?}", screen.texts);
    }
    // The header's buttons are nested components: their events change the
    // header's state.
    common::press(&engine, &rx, &mut screen, "Tap");
    common::press(&engine, &rx, &mut screen, "Tap");
    let taps = screen.find("Taps").unwrap();
    let row = screen.parents[&taps];
    let count = screen
        .texts
        .iter()
        .find(|t| t.0 != taps && screen.parents.get(&t.0) == Some(&row))
        .map(|t| t.1.clone());
    assert_eq!(count.as_deref(), Some("2"), "{:?}", screen.texts);

    // The select list's rows are the select list's own.
    common::press(&engine, &rx, &mut screen, "City 3");
    assert!(
        shows(&screen, "SelectList: 500 rows of Rust data, chosen: City 3"),
        "{:?}",
        screen.texts
    );

    common::press(&engine, &rx, &mut screen, "Ask the stepper, then reset it");
    assert!(shows(&screen, "It was 5; reset to 5"), "{:?}", screen.texts);

    common::press(&engine, &rx, &mut screen, "Inset rows");
    assert!(shows(&screen, "Plain rows"), "{:?}", screen.texts);
}
