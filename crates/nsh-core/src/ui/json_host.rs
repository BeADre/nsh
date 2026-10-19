//! A JSON-value host shared by native widget props and the core tests. It reads and makes
//! values as `nsh-js` does with QuickJS's, in an arena of JSON values
//! (`undefined` apart), so props, events and results convert as they do
//! for an app.

use std::{cell::RefCell, rc::Rc};

use super::{
    AnyValue, CallbackHandle, ClassInstance, JsHost, JsIn, JsKind, JsOwned, JsRaw, PropDecode,
    PropError,
};
use crate::{JsonValue, NodeId};

#[derive(Clone)]
enum Val {
    Undefined,
    Json(Rc<JsonValue>),
}

/// The host: its values live as long as it does.
#[derive(Default)]
pub(crate) struct JsonHost {
    values: RefCell<Vec<Val>>,
    args: Vec<JsonValue>,
    #[cfg(test)]
    copied_nodes: std::cell::Cell<usize>,
}

impl JsonHost {
    /// A host whose call has `args`.
    #[cfg(test)]
    pub(crate) fn with_args(args: Vec<JsonValue>) -> Self {
        Self {
            values: RefCell::default(),
            args,
            copied_nodes: std::cell::Cell::new(0),
        }
    }

    fn put(&self, v: Val) -> JsRaw {
        let mut values = self.values.borrow_mut();
        values.push(v);
        JsRaw([(values.len() - 1) as u64, 0])
    }

    fn val(&self, v: JsRaw) -> Val {
        self.values.borrow()[v.0[0] as usize].clone()
    }

    fn json_of(&self, v: JsRaw) -> Option<Rc<JsonValue>> {
        match self.val(v) {
            Val::Undefined => None,
            Val::Json(json) => Some(json),
        }
    }

    // Reads clone only the selected child, never its containing array/object.
    fn copy_json(&self, value: &JsonValue) -> JsonValue {
        #[cfg(test)]
        {
            fn nodes(value: &JsonValue) -> usize {
                1 + match value {
                    JsonValue::Array(values) => values.iter().map(nodes).sum(),
                    JsonValue::Object(values) => values.values().map(nodes).sum(),
                    _ => 0,
                }
            }
            self.copied_nodes
                .set(self.copied_nodes.get() + nodes(value));
        }
        value.clone()
    }

    /// `json` as a value of the host.
    pub(crate) fn value(&self, json: JsonValue) -> JsRaw {
        self.put(Val::Json(Rc::new(json)))
    }

    /// What `v` is (`None`: `undefined`).
    pub(crate) fn read(&self, v: JsOwned<'_>) -> Option<JsonValue> {
        self.json_of(v.into_raw())
            .map(|value| self.copy_json(&value))
    }
}

/// Prop value `json` as `decode` reads it from JS.
pub(crate) fn decode(decode: PropDecode, json: JsonValue) -> Result<AnyValue, PropError> {
    let host = JsonHost::default();
    let raw = host.value(json);
    decode(JsIn::new(&host, raw))
}

/// The data JS hears with `event` (`None`: none).
#[cfg(test)]
pub(crate) fn event_data(event: &dyn super::EventToJs) -> Option<JsonValue> {
    let host = JsonHost::default();
    let data = event.data_js(&host).expect("the event's data converts");
    host.read(data)
}

/// The value a call's result settles its promise with (`None`:
/// `undefined`), or the rejection's message.
#[cfg(test)]
pub(crate) fn result(result: super::JsResult) -> Result<Option<JsonValue>, String> {
    let host = JsonHost::default();
    let value = result?.to_js_value(&host).map_err(|e| e.0)?;
    Ok(host.read(value))
}

impl JsHost for JsonHost {
    fn kind(&self, v: JsRaw) -> JsKind {
        match self.val(v) {
            Val::Undefined => JsKind::Undefined,
            Val::Json(value) => match value.as_ref() {
                JsonValue::Null => JsKind::Null,
                JsonValue::Bool(_) => JsKind::Bool,
                JsonValue::Number(_) => JsKind::Number,
                JsonValue::String(_) => JsKind::String,
                JsonValue::Array(_) => JsKind::Array,
                JsonValue::Object(_) => JsKind::Object,
            },
        }
    }

    fn bool(&self, v: JsRaw) -> Option<bool> {
        self.json_of(v)?.as_bool()
    }

    fn number(&self, v: JsRaw) -> Option<f64> {
        self.json_of(v)?.as_f64()
    }

    fn string(&self, v: JsRaw) -> Option<String> {
        self.json_of(v)?.as_str().map(str::to_owned)
    }

    fn len(&self, v: JsRaw) -> Result<Option<usize>, PropError> {
        Ok(self.json_of(v).and_then(|j| j.as_array().map(Vec::len)))
    }

    fn index(&self, v: JsRaw, i: usize) -> Result<JsRaw, PropError> {
        let item = self
            .json_of(v)
            .and_then(|j| j.get(i).map(|value| Rc::new(self.copy_json(value))));
        Ok(self.put(item.map_or(Val::Undefined, Val::Json)))
    }

    fn field(&self, v: JsRaw, key: &str) -> Result<JsRaw, PropError> {
        let field = self
            .json_of(v)
            .and_then(|j| j.get(key).map(|value| Rc::new(self.copy_json(value))));
        Ok(self.put(field.map_or(Val::Undefined, Val::Json)))
    }

    fn to_json(&self, v: JsRaw) -> Result<JsonValue, PropError> {
        Ok(self
            .json_of(v)
            .map_or(JsonValue::Null, |value| self.copy_json(&value)))
    }

    fn describe(&self, v: JsRaw) -> String {
        match self.val(v) {
            Val::Undefined => "undefined".to_owned(),
            Val::Json(json) => json.to_string(),
        }
    }

    /// Objects' numbers, strings and nulls, as the core once read inline
    /// styles (enough for its tests).
    fn style(&self, v: JsRaw) -> Result<crate::StylePatch, PropError> {
        fn walk(value: &JsonValue, style: &mut crate::Style, patch: &mut crate::StylePatch) {
            use crate::StyleValue as V;
            match value {
                JsonValue::Object(entries) => {
                    for (key, value) in entries {
                        let v = match value {
                            JsonValue::Number(n) => V::Number(n.as_f64().unwrap_or(0.0) as f32),
                            JsonValue::String(s) => V::Str(s),
                            JsonValue::Null => V::Null,
                            _ => continue,
                        };
                        let _ = patch.set(style, key, v);
                    }
                }
                JsonValue::Array(items) => {
                    for item in items {
                        walk(item, style, patch);
                    }
                }
                _ => {}
            }
        }
        let mut style = crate::Style::default();
        let mut patch = crate::StylePatch::default();
        if let Some(json) = self.json_of(v) {
            walk(&json, &mut style, &mut patch);
        }
        Ok(patch)
    }

    fn node(&self, _v: JsRaw) -> Option<NodeId> {
        None
    }

    fn node_value(&self, _id: NodeId) -> JsRaw {
        self.null()
    }

    fn instance(&self, _v: JsRaw) -> Option<Rc<dyn ClassInstance>> {
        None
    }

    fn callback(&self, _v: JsRaw) -> Option<Rc<dyn CallbackHandle>> {
        None
    }

    fn free(&self, _v: JsRaw) {}

    fn undefined(&self) -> JsRaw {
        self.put(Val::Undefined)
    }

    fn null(&self) -> JsRaw {
        self.value(JsonValue::Null)
    }

    fn new_bool(&self, b: bool) -> JsRaw {
        self.value(JsonValue::Bool(b))
    }

    /// A whole number reads as an integer, as `JSON.stringify` writes it.
    fn new_number(&self, n: f64) -> JsRaw {
        if n.fract() == 0.0 && n.abs() < 2f64.powi(53) {
            return self.value(serde_json::json!(n as i64));
        }
        self.value(serde_json::json!(n))
    }

    fn new_string(&self, s: &str) -> Result<JsRaw, PropError> {
        Ok(self.value(JsonValue::String(s.to_owned())))
    }

    fn new_array(&self, items: Vec<JsRaw>) -> Result<JsRaw, PropError> {
        let items = items
            .into_iter()
            .map(|i| {
                self.json_of(i)
                    .map_or(JsonValue::Null, |value| self.copy_json(&value))
            })
            .collect();
        Ok(self.value(JsonValue::Array(items)))
    }

    fn new_object(&self) -> Result<JsRaw, PropError> {
        Ok(self.value(JsonValue::Object(Default::default())))
    }

    fn set_field(&self, obj: JsRaw, key: &str, value: JsRaw) -> Result<(), PropError> {
        // `undefined` fields are left out, as `JSON.stringify` does.
        let Some(value) = self.json_of(value) else {
            return Ok(());
        };
        if let Val::Json(object) = &mut self.values.borrow_mut()[obj.0[0] as usize]
            && let JsonValue::Object(map) = Rc::make_mut(object)
        {
            map.insert(key.to_owned(), self.copy_json(&value));
        }
        Ok(())
    }

    fn json(&self, v: &JsonValue) -> Result<JsRaw, PropError> {
        Ok(self.value(v.clone()))
    }

    fn wrap(&self, _instance: Rc<dyn ClassInstance>) -> Result<JsRaw, PropError> {
        Err(PropError("the JSON host has no classes".to_owned()))
    }

    fn arg(&self, i: usize) -> JsRaw {
        match self.args.get(i) {
            Some(json) => self.value(json.clone()),
            None => self.undefined(),
        }
    }

    fn this(&self) -> JsRaw {
        self.undefined()
    }

    fn construct(&self, _instance: Rc<dyn ClassInstance>) -> Result<JsRaw, PropError> {
        Err(PropError("the JSON host has no classes".to_owned()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::JsValue;
    #[test]
    fn large_array_decode_copies_only_selected_values() {
        let host = JsonHost::default();
        let count = 8192;
        let value = host.value(serde_json::json!((0..count).collect::<Vec<_>>()));
        let first = host.json_of(value).unwrap();
        for _ in 0..32 {
            assert_eq!(host.len(value).unwrap(), Some(count));
            assert_eq!(host.kind(value), JsKind::Array);
            assert!(Rc::ptr_eq(&first, &host.json_of(value).unwrap()));
        }
        assert_eq!(host.copied_nodes.get(), 0);
        let decoded = Vec::<f64>::from_js(JsIn::new(&host, value)).unwrap();
        assert_eq!(decoded.len(), count);
        assert_eq!(
            host.copied_nodes.get(),
            count,
            "one scalar copy per element, not one entire array per element"
        );
    }
}
