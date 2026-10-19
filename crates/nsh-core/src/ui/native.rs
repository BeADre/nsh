//! Typed JSON props shared by platform-native widget hosts.

use std::collections::HashSet;

use super::{PropError, PropKind, PropSpec, Props, TsTypes};
use crate::JsonValue;

/// A native widget's accepted props and its pending update batch. Conversion
/// uses the same `Props` decoder as Rust UI; no parallel reflection schema.
pub struct NativeProps<P: Props> {
    accepted: P,
    pending: Option<P>,
    pending_error: Option<PropError>,
    fields: Vec<PropSpec>,
    provided: HashSet<String>,
    candidate_provided: HashSet<String>,
    dirty: Vec<String>,
}
impl<P: Props> Default for NativeProps<P> {
    fn default() -> Self {
        Self::new()
    }
}
impl<P: Props> NativeProps<P> {
    /// Starts with the derived defaults, with no pending update.
    pub fn new() -> Self {
        assert!(
            !P::HAS_REACTIVE_FIELDS,
            "native props must use plain value fields, not owner-bound Prop<T>"
        );
        Self {
            accepted: P::initial(),
            pending: None,
            pending_error: None,
            fields: P::spec(&mut TsTypes::default()),
            provided: HashSet::new(),
            candidate_provided: HashSet::new(),
            dirty: Vec::new(),
        }
    }
    /// The last complete, valid batch. Widgets never observe a partially
    /// applied ownership switch or an invalid exclusive batch.
    pub fn props(&self) -> &P {
        &self.accepted
    }
    /// Stages a value (`None` restores its derived default). Known slot and
    /// children fields are tree metadata, never serialized UI values.
    pub fn set_json(&mut self, name: &str, value: Option<&JsonValue>) -> Result<(), PropError> {
        let result = self.stage_json(name, value);
        if let Err(error) = &result {
            self.pending_error
                .get_or_insert_with(|| PropError(format!("{name}: {error}")));
        }
        result
    }
    fn stage_json(&mut self, name: &str, value: Option<&JsonValue>) -> Result<(), PropError> {
        let field = self
            .fields
            .iter()
            .find(|field| field.name == name)
            .ok_or_else(|| PropError(format!("unknown native value prop `{name}`")))?;
        if field.kind != PropKind::Value {
            return Ok(());
        }
        let decoded = value
            .map(|value| {
                let decode = field
                    .decode
                    .ok_or_else(|| PropError(format!("native prop `{name}` has no decoder")))?;
                super::decode_json(decode, value.clone())
            })
            .transpose()?;
        self.pending
            .get_or_insert_with(|| self.accepted.clone())
            .set(name, decoded)?;
        if value.is_some_and(|value| !value.is_null()) {
            self.candidate_provided.insert(name.to_owned());
        } else {
            self.candidate_provided.remove(name);
        }
        // A same-value acknowledgment may need to undo a widget's rejected
        // edit. Equality in Props must not suppress this lifecycle hook.
        if !self.dirty.iter().any(|field| field == name) {
            self.dirty.push(name.to_owned());
        }
        Ok(())
    }
    /// Commits before rendering or a command, returning the delivered fields
    /// in order. An invalid batch emits one error and retains accepted props;
    /// calling again without a new batch succeeds without repeating it.
    pub fn commit(&mut self) -> Result<Vec<String>, PropError> {
        self.commit_with(|_| Ok(()))
    }
    /// Validates component-specific domain rules before accepting a complete
    /// batch. Rejected candidates are discarded and never reach widget hooks.
    pub fn commit_with(
        &mut self,
        validate: impl FnOnce(&P) -> Result<(), PropError>,
    ) -> Result<Vec<String>, PropError> {
        if let Some(error) = self.pending_error.take() {
            self.pending = None;
            self.dirty.clear();
            self.candidate_provided = self.provided.clone();
            return Err(error);
        }
        let Some(candidate) = self.pending.take() else {
            return Ok(Vec::new());
        };
        let dirty = std::mem::take(&mut self.dirty);
        for group in P::EXCLUSIVE {
            if group
                .iter()
                .filter(|field| self.candidate_provided.contains(**field))
                .count()
                > 1
            {
                self.candidate_provided = self.provided.clone();
                return Err(PropError(format!(
                    "mutually exclusive props: {}",
                    group.join(", ")
                )));
            }
        }
        if let Err(error) = validate(&candidate) {
            self.candidate_provided = self.provided.clone();
            return Err(error);
        }
        self.accepted = candidate;
        self.provided = self.candidate_provided.clone();
        Ok(dirty)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::ui::{Children, PropKind, Slot};

    macro_rules! forwarded_props {
        ($($name:ident:$ty:ty),* $(,)?) => {
            #[derive(Clone, PartialEq, crate::ui::Props)]
            struct Forwarded { $($name:$ty,)* }
        };
    }
    forwarded_props!(value:Option<String>, header:Option<Slot>, children:Children);

    #[derive(Clone, PartialEq, crate::ui::Props)]
    struct Editor {
        #[prop(exclusive = "content")]
        value: Option<String>,
        content: Option<String>,
    }
    #[test]
    fn forwarded_types_keep_optional_and_live_slot_metadata() {
        let fields = Forwarded::spec(&mut TsTypes::default());
        assert!(
            fields
                .iter()
                .find(|field| field.name == "value")
                .unwrap()
                .optional
        );
        assert_eq!(
            fields
                .iter()
                .find(|field| field.name == "header")
                .unwrap()
                .kind,
            PropKind::Slot
        );
        assert_eq!(
            fields
                .iter()
                .find(|field| field.name == "children")
                .unwrap()
                .kind,
            PropKind::Children
        );
    }
    #[test]
    fn complete_batches_allow_ownership_switches_and_preserve_same_value_ack() {
        let mut props = NativeProps::<Editor>::new();
        props.set_json("value", Some(&json!("first"))).unwrap();
        assert!(props.props().value.is_none());
        assert_eq!(props.commit().unwrap(), ["value"]);
        // The new owner may arrive before the old owner is cleared.
        props.set_json("content", Some(&json!("rich"))).unwrap();
        props.set_json("value", Some(&JsonValue::Null)).unwrap();
        assert_eq!(props.commit().unwrap(), ["content", "value"]);
        assert_eq!(props.props().content.as_deref(), Some("rich"));
        props.set_json("content", Some(&json!("rich"))).unwrap();
        assert_eq!(props.commit().unwrap(), ["content"]);
    }
    #[test]
    fn invalid_batch_reports_once_and_never_poison_accepted_props() {
        let mut props = NativeProps::<Editor>::new();
        props.set_json("value", Some(&json!("accepted"))).unwrap();
        props.commit().unwrap();
        props.set_json("content", Some(&json!("conflict"))).unwrap();
        assert!(props.commit().is_err());
        assert_eq!(props.props().value.as_deref(), Some("accepted"));
        assert!(props.props().content.is_none());
        assert!(props.commit().unwrap().is_empty());
        props
            .set_json("content", Some(&json!("corrected")))
            .unwrap();
        props.set_json("value", None).unwrap();
        props.commit().unwrap();
        assert_eq!(props.props().content.as_deref(), Some("corrected"));
    }
    #[test]
    fn command_boundaries_observe_only_preceding_updates() {
        let mut props = NativeProps::<Editor>::new();
        props
            .set_json("value", Some(&json!("before command")))
            .unwrap();
        props.commit().unwrap();
        let command_value = props.props().value.clone();
        props
            .set_json("value", Some(&json!("after command")))
            .unwrap();
        assert_eq!(command_value.as_deref(), Some("before command"));
        assert_eq!(props.props().value.as_deref(), Some("before command"));
        props.commit().unwrap();
        assert_eq!(props.props().value.as_deref(), Some("after command"));
    }
    #[derive(Clone, PartialEq, crate::ui::Props)]
    struct ReactiveFields {
        value: crate::ui::Prop<String>,
    }

    #[test]
    #[should_panic(expected = "plain value fields")]
    fn native_snapshots_reject_owner_bound_fields_before_initialization() {
        NativeProps::<ReactiveFields>::new();
    }

    #[test]
    fn native_registry_reuses_specs_without_a_rust_view_and_emits_constraints() {
        let mut registry = crate::ui::Registry::ui_set("test/native");
        registry.register_native::<Editor, crate::ui::NoEvent>("test.Editor", "Editor");
        assert!(registry.is_native("test.Editor"));
        assert!(registry.make("test.Editor").is_none());
        let declarations = crate::ui::declarations(&registry, &[]);
        assert!(declarations.contains("export type EditorProps = EditorBaseProps & ("));
        assert!(declarations.contains("value?: never"));
        assert!(declarations.contains("content?: never"));
        assert!(!declarations.contains("Promise<"));
    }
    #[test]
    fn fixed_arrays_have_exact_tuple_types_and_lengths() {
        use crate::ui::{JsValue, TsTypes};
        let host = crate::ui::json_host::JsonHost::default();
        let raw = host.value(json!([2024, 2030]));
        let value = <[u32; 2]>::from_js(crate::ui::JsIn::new(&host, raw)).unwrap();
        assert_eq!(value, [2024, 2030]);
        assert_eq!(
            <[u32; 2]>::ts_type(&mut TsTypes::default()),
            "[number, number]"
        );
        assert_eq!(
            host.read(value.to_js(&host).unwrap()),
            Some(json!([2024, 2030]))
        );
        for json in [json!([]), json!([2024]), json!([2024, 2025, 2026])] {
            let raw = host.value(json);
            assert!(<[u32; 2]>::from_js(crate::ui::JsIn::new(&host, raw)).is_err());
        }
    }
    #[test]
    fn dynamic_json_records_require_objects_and_preserve_record_types() {
        use crate::ui::{JsValue, TsTypes};
        type Record = serde_json::Map<String, JsonValue>;
        let host = crate::ui::json_host::JsonHost::default();
        let raw = host.value(json!({"name":"Ada", "score":42}));
        let value = Record::from_js(crate::ui::JsIn::new(&host, raw)).unwrap();
        assert_eq!(value["name"], "Ada");
        assert_eq!(
            Record::ts_type(&mut TsTypes::default()),
            "Record<string, unknown>"
        );
        assert_eq!(
            host.read(value.to_js(&host).unwrap()),
            Some(json!({"name":"Ada", "score":42}))
        );
        let raw = host.value(json!([1, 2]));
        assert!(Record::from_js(crate::ui::JsIn::new(&host, raw)).is_err());
    }
    #[test]
    fn readonly_inputs_delegate_values_without_changing_mutable_vec_types() {
        use crate::ui::{JsValue, Readonly, TsTypes};
        let host = crate::ui::json_host::JsonHost::default();
        let raw = host.value(json!([1, 2]));
        let value = Readonly::<Vec<u32>>::from_js(crate::ui::JsIn::new(&host, raw)).unwrap();
        assert_eq!(value.iter().copied().collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(
            Readonly::<Vec<u32>>::ts_type(&mut TsTypes::default()),
            "Readonly<(number)[]>"
        );
        assert_eq!(Vec::<u32>::ts_type(&mut TsTypes::default()), "(number)[]");
        assert_eq!(host.read(value.to_js(&host).unwrap()), Some(json!([1, 2])));
        assert_eq!(value.into_inner(), vec![1, 2]);
    }
    #[test]
    fn typed_records_decode_each_field_and_reject_invalid_values() {
        use crate::ui::{JsValue, TsTypes};
        type Record = std::collections::BTreeMap<String, u32>;
        let host = crate::ui::json_host::JsonHost::default();
        let raw = host.value(json!({"alpha":1, "beta":2}));
        let value = Record::from_js(crate::ui::JsIn::new(&host, raw)).unwrap();
        assert_eq!(
            value.keys().map(String::as_str).collect::<Vec<_>>(),
            ["alpha", "beta"]
        );
        assert_eq!(value["beta"], 2);
        assert_eq!(
            Record::ts_type(&mut TsTypes::default()),
            "Record<string, number>"
        );
        assert_eq!(
            host.read(value.to_js(&host).unwrap()),
            Some(json!({"alpha":1, "beta":2}))
        );
        let raw = host.value(json!([1, 2]));
        assert!(Record::from_js(crate::ui::JsIn::new(&host, raw)).is_err());
        let raw = host.value(json!({"broken":"text"}));
        assert!(
            Record::from_js(crate::ui::JsIn::new(&host, raw))
                .unwrap_err()
                .to_string()
                .contains("broken:")
        );
    }
    #[test]
    fn rejected_domain_batches_do_not_poison_later_updates_or_repeat_errors() {
        let mut props = NativeProps::<Editor>::new();
        props.set_json("value", Some(&json!("valid"))).unwrap();
        props.commit().unwrap();
        props.set_json("value", Some(&json!("invalid"))).unwrap();
        let validate = |candidate: &Editor| {
            if candidate.value.as_deref() == Some("invalid") {
                Err(PropError("invalid domain".into()))
            } else {
                Ok(())
            }
        };
        assert!(props.commit_with(validate).is_err());
        assert_eq!(props.props().value.as_deref(), Some("valid"));
        assert!(
            props
                .commit_with(|_| panic!("no pending candidate"))
                .unwrap()
                .is_empty()
        );
        props.set_json("content", None).unwrap();
        assert_eq!(props.commit_with(validate).unwrap(), ["content"]);
        assert_eq!(props.props().value.as_deref(), Some("valid"));
    }
    #[test]
    fn boxed_values_preserve_wire_type_and_decoder_errors() {
        use crate::ui::{JsIn, JsValue};
        let host = crate::ui::json_host::JsonHost::default();
        let json = json!(["one", "two"]);
        let value =
            Box::<Vec<String>>::from_js(JsIn::new(&host, host.value(json.clone()))).unwrap();
        assert_eq!(value.as_ref(), &["one", "two"]);
        assert_eq!(host.read(value.to_js(&host).unwrap()), Some(json));
        assert_eq!(
            Box::<Vec<String>>::ts_type(&mut TsTypes::default()),
            Vec::<String>::ts_type(&mut TsTypes::default())
        );
        assert!(Box::<Vec<String>>::from_js(JsIn::new(&host, host.value(json!([42])))).is_err());
    }

    #[test]
    fn byte_values_use_strict_unsigned_integer_bounds() {
        use crate::ui::JsValue;
        let host = crate::ui::json_host::JsonHost::default();
        for (json, expected) in [(json!(0), 0), (json!(255), 255)] {
            let raw = host.value(json);
            assert_eq!(
                u8::from_js(crate::ui::JsIn::new(&host, raw)).unwrap(),
                expected
            );
        }
        for json in [json!(-1), json!(256), json!(1.5), json!("1")] {
            let raw = host.value(json);
            assert!(u8::from_js(crate::ui::JsIn::new(&host, raw)).is_err());
        }
    }
    #[derive(Clone, PartialEq, crate::ui::Props)]
    struct Transactional {
        value: Option<String>,
        #[prop(default)]
        disabled: bool,
    }

    #[test]
    fn decode_invalid_batches_reject_all_fields_once_and_recover() {
        let mut props = NativeProps::<Transactional>::new();
        props.set_json("value", Some(&json!("accepted"))).unwrap();
        props.commit().unwrap();
        assert!(props.set_json("value", Some(&json!(42))).is_err());
        props.set_json("disabled", Some(&json!(true))).unwrap();
        assert!(
            props.commit().is_err(),
            "a decoder error must reject the entire batch"
        );
        assert_eq!(props.props().value.as_deref(), Some("accepted"));
        assert!(!props.props().disabled);
        assert!(
            props.commit().unwrap().is_empty(),
            "one batch reports one error"
        );
        props.set_json("value", Some(&json!("recovered"))).unwrap();
        props.set_json("disabled", Some(&json!(true))).unwrap();
        assert_eq!(props.commit().unwrap(), ["value", "disabled"]);
        assert_eq!(props.props().value.as_deref(), Some("recovered"));
        assert!(props.props().disabled);
    }

    #[test]
    fn invalid_only_and_unknown_prop_batches_fail_once_before_commands() {
        let mut props = NativeProps::<Transactional>::new();
        for field in ["value", "unknown"] {
            assert!(props.set_json(field, Some(&json!(42))).is_err());
            assert!(
                props.commit().is_err(),
                "invalid-only batch has no staged Props but must fail"
            );
            assert!(props.commit().unwrap().is_empty());
            assert_eq!(props.props().value, None);
        }
    }
}
