//! The native UI set uses NSH's real renderer, owners and event transport.
use nsh_core::{
    Command, EventKind, EventPayload, Op, Prop, Tag,
    ui::{self, Registry},
};
use nsh_js::{Host, HostOptions};
use serde_json::json;

#[derive(Clone, PartialEq, ui::Props)]
struct ProbeProps {
    value: Option<String>,
    header: Option<ui::Slot>,
    children: ui::Children,
}
#[derive(ui::Event)]
enum ProbeEvent {
    Change(String),
}

fn take(host: &Host) -> Vec<Op> {
    let mut out = Vec::new();
    host.take_ops(&mut out);
    out
}

#[test]
fn native_slots_keep_their_owners_across_record_updates_and_acknowledge_vetoes() {
    assert_eq!(
        ui::event_json(&ProbeEvent::Change("payload".into())).unwrap(),
        Some(json!("payload"))
    );
    let mut registry = Registry::ui_set("test/native");
    registry
        .register_native::<ProbeProps, ProbeEvent>("test.native.Probe", "Probe")
        .native_controlled("test.native.Probe", &[("change", &["value"])]);
    let host = Host::with_options(HostOptions {
        ui_sets: vec![registry],
        ..Default::default()
    })
    .unwrap();
    host.eval_source(r#"
        import { Probe } from 'test/native';
        import { mount, signal } from 'nsh';
        import { element, insert } from 'nsh:rt';
        const revision = signal(0);
        const first = element('view'), second = element('view'), footer = element('view');
        insert(first, 'first'); insert(second, 'second'); insert(footer, 'footer');
        const content = signal(first);
        globalThis.bump = () => revision.set(1);
        globalThis.replace = () => content.set(second);
        mount(() => Probe({
            value: 'accepted',
            get slots() { revision(); return { header: content(), ...(revision() ? { footer } : {}) }; },
            onChange(value) { globalThis.seen = value; },
        }));
    "#).unwrap();
    let initial = take(&host);
    let native = |name: &str| {
        initial
            .iter()
            .find_map(|op| match op {
                Op::SetProp {
                    id,
                    prop: Prop::Component(component),
                } if component.as_ref() == name => Some(*id),
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing {name}: {initial:?}"))
    };
    let parent = native("test.native.Probe");
    let header = native("__nsh.Slot");
    assert!(
        initial
            .iter()
            .filter_map(|op| match op {
                Op::SetStyle { id, style, .. } if *id == header => Some(style),
                _ => None,
            })
            .all(|style| style.layout.size.width.is_auto()),
        "an unmeasured slot must not impose zero width on ordinary NSH content"
    );

    host.eval_source("globalThis.bump();").unwrap();
    let changed = take(&host);
    assert_eq!(
        changed
            .iter()
            .filter(|op| matches!(
                op,
                Op::Create {
                    tag: Tag::Native,
                    ..
                }
            ))
            .count(),
        1,
        "only the new footer creates a slot; the header retains identity"
    );
    host.eval_source("globalThis.replace();").unwrap();
    let replaced = take(&host);
    assert!(
        replaced
            .iter()
            .any(|op| matches!(op, Op::Insert { parent, .. } if *parent == header)),
        "the retained header slot's child effect must still react: {replaced:?}"
    );

    host.dispatch_event(
        header,
        EventKind::NativeEvent,
        &EventPayload::Native {
            name: "measure".into(),
            data: json!({"width":177}),
        },
    );
    let measured = take(&host);
    assert!(
        measured
            .iter()
            .any(|op| matches!(op, Op::SetStyle { id, .. } if *id == header)),
        "the retained slot's width signal must still react: {measured:?}"
    );

    host.dispatch_event(
        parent,
        EventKind::NativeEvent,
        &EventPayload::Native {
            name: "change".into(),
            data: json!("rejected"),
        },
    );
    let events = take(&host);
    assert!(events.iter().any(|op| matches!(op, Op::Command { id, command: Command::Native { name, args } }
        if *id == parent && name.as_ref() == "__nsh.controlled_ack" && args == &json!({"value":"accepted"}))),
        "a same-value owner veto is acknowledged after the real handler: {events:?}");
}

#[test]
fn slot_getter_constructs_once_per_update_and_cleans_up_once() {
    let mut registry = Registry::ui_set("test/native");
    registry.register_native::<ProbeProps, ProbeEvent>("test.native.Probe", "Probe");
    let host = Host::with_options(HostOptions {
        ui_sets: vec![registry],
        ..Default::default()
    })
    .unwrap();
    host.eval_source(
        r#"
        import { Probe } from 'test/native';
        import { mount, signal, cleanup } from 'nsh';
        import { element, insert, text } from 'nsh:rt';
        const revision = signal(0), shown = signal(true);
        let made = 0, cleaned = 0;
        globalThis.next = () => revision.update(n => n + 1);
        globalThis.hide = () => shown.set(false);
        globalThis.report = () => text(`${made}/${cleaned}`);
        mount(() => Probe({
            get slots() {
                const current = revision();
                if (!shown()) return {};
                made++; cleanup(() => cleaned++);
                const node = element('view');
                insert(node, `factory ${current}`);
                return { header: node };
            },
        }));
        globalThis.report();
    "#,
    )
    .unwrap();
    let reported = |host: &Host| {
        take(host)
            .into_iter()
            .filter_map(|op| match op {
                Op::CreateText { text, .. } => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(reported(&host), ["factory 0", "1/0"]);
    host.eval_source("globalThis.next(); globalThis.report();")
        .unwrap();
    assert_eq!(reported(&host), ["factory 1", "2/1"]);
    host.eval_source("globalThis.next(); globalThis.report();")
        .unwrap();
    assert_eq!(reported(&host), ["factory 2", "3/2"]);
    host.eval_source("globalThis.hide(); globalThis.report();")
        .unwrap();
    assert_eq!(reported(&host), ["3/3"]);
}

#[test]
fn compatibility_exports_share_composition_and_keep_raw_factories_as_leaves() {
    let host = Host::new().unwrap();
    host.eval_source(r#"
        import { componentWithChildren, nativeComponent, Slot } from 'nsh/native';
        import { mount } from 'nsh';
        import { element, insert } from 'nsh:rt';
        const Parent = componentWithChildren('test.Parent', { props: ['value'], events: ['change'] });
        const Raw = nativeComponent('test.Raw', { props: [] });
        const child = element('view'); insert(child, 'shared subtree');
        mount(() => Parent({ value: 'accepted', children: Slot({name: 'header', children: child}), onChange() {} }));
        globalThis.raw = () => Raw({children: element('view')});
    "#).unwrap();
    let ops = take(&host);
    let parent = ops
        .iter()
        .find_map(|op| match op {
            Op::SetProp {
                id,
                prop: Prop::Component(name),
            } if name.as_ref() == "test.Parent" => Some(*id),
            _ => None,
        })
        .unwrap();
    assert!(ops.iter().any(|op| matches!(op, Op::SetProp { prop: Prop::Component(name), .. } if name.as_ref() == "__nsh.Slot")));
    host.dispatch_event(
        parent,
        EventKind::NativeEvent,
        &EventPayload::Native {
            name: "change".into(),
            data: json!("rejected"),
        },
    );
    assert!(take(&host).iter().any(|op| matches!(op, Op::Command { command: Command::Native { name, args }, .. } if name.as_ref() == "__nsh.controlled_ack" && args == &json!({"value":"accepted"}))));
    host.eval_source("globalThis.raw();").unwrap();
    let raw_ops = take(&host);
    let raw = raw_ops
        .iter()
        .find_map(|op| match op {
            Op::SetProp {
                id,
                prop: Prop::Component(name),
            } if name.as_ref() == "test.Raw" => Some(*id),
            _ => None,
        })
        .unwrap();
    assert!(
        !raw_ops
            .iter()
            .any(|op| matches!(op, Op::Insert {parent, ..} if *parent == raw))
    );
}
