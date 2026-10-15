//! Values crossing between JS and components: shared strings, and the
//! conversion of prop and event values from and to JSON with their
//! TypeScript types.

use std::{borrow::Borrow, fmt, ops::Deref, sync::Arc};

use crate::{JsonValue, style::Color};

/// A cheaply cloned string: a static string or a shared one.
#[derive(Clone)]
pub struct SharedString(Repr);

#[derive(Clone)]
enum Repr {
    Static(&'static str),
    Shared(Arc<str>),
}

impl SharedString {
    /// A static string, without allocating.
    pub const fn new_static(s: &'static str) -> Self {
        Self(Repr::Static(s))
    }

    /// The string.
    pub fn as_str(&self) -> &str {
        match &self.0 {
            Repr::Static(s) => s,
            Repr::Shared(s) => s,
        }
    }

    pub(crate) fn to_arc(&self) -> Arc<str> {
        match &self.0 {
            Repr::Static(s) => Arc::from(*s),
            Repr::Shared(s) => s.clone(),
        }
    }
}

impl Default for SharedString {
    fn default() -> Self {
        Self::new_static("")
    }
}

impl Deref for SharedString {
    type Target = str;

    fn deref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<str> for SharedString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for SharedString {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl PartialEq for SharedString {
    fn eq(&self, other: &Self) -> bool {
        let (a, b) = (self.as_str(), other.as_str());
        std::ptr::eq(a, b) || a == b
    }
}

impl Eq for SharedString {}

impl std::hash::Hash for SharedString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_str().hash(state)
    }
}

impl PartialEq<str> for SharedString {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for SharedString {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl fmt::Debug for SharedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), f)
    }
}

impl fmt::Display for SharedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&'static str> for SharedString {
    fn from(s: &'static str) -> Self {
        Self::new_static(s)
    }
}

impl From<String> for SharedString {
    fn from(s: String) -> Self {
        Self(Repr::Shared(s.into()))
    }
}

impl From<Arc<str>> for SharedString {
    fn from(s: Arc<str>) -> Self {
        Self(Repr::Shared(s))
    }
}

impl From<&SharedString> for SharedString {
    fn from(s: &SharedString) -> Self {
        s.clone()
    }
}

/// A prop value JS sent that does not fit the prop's type.
#[derive(Debug, Clone, PartialEq)]
pub struct PropError(pub String);

impl PropError {
    /// "expected `ty`, got `value`".
    pub fn expected(ty: &str, value: &JsonValue) -> Self {
        let mut got = value.to_string();
        if got.len() > 40 {
            got.truncate(40);
            got.push('…');
        }
        Self(format!("expected {ty}, got {got}"))
    }
}

impl fmt::Display for PropError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for PropError {}

/// A type props and event data can have: how it converts from and to JSON,
/// and its TypeScript type. `#[derive(PropEnum)]` implements it for enums
/// of unit variants.
pub trait JsValue: Sized {
    /// Its TypeScript type (`string`, `"primary" | "plain"`).
    fn ts_type() -> String;

    /// The value JS sent.
    fn from_json(value: JsonValue) -> Result<Self, PropError>;

    /// The value for JS.
    fn to_json(&self) -> JsonValue;
}

impl JsValue for bool {
    fn ts_type() -> String {
        "boolean".into()
    }

    fn from_json(value: JsonValue) -> Result<Self, PropError> {
        value
            .as_bool()
            .ok_or_else(|| PropError::expected("a boolean", &value))
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::Bool(*self)
    }
}

macro_rules! number {
    ($to_json:ident: $($ty:ty),*) => {$(
        impl JsValue for $ty {
            fn ts_type() -> String {
                "number".into()
            }

            fn from_json(value: JsonValue) -> Result<Self, PropError> {
                value
                    .as_f64()
                    .map(|n| n as $ty)
                    .ok_or_else(|| PropError::expected("a number", &value))
            }

            fn to_json(&self) -> JsonValue {
                $to_json(*self)
            }
        }
    )*};
}

fn float(n: impl Into<f64>) -> JsonValue {
    serde_json::Number::from_f64(n.into()).map_or(JsonValue::Null, JsonValue::Number)
}

fn int(n: impl Into<i64>) -> JsonValue {
    JsonValue::Number(n.into().into())
}

fn uint(n: impl TryInto<u64>) -> JsonValue {
    n.try_into()
        .map_or(JsonValue::Null, |n: u64| JsonValue::Number(n.into()))
}

number!(float: f32, f64);
number!(int: i32, i64);
number!(uint: u32, u64, usize);

impl JsValue for SharedString {
    fn ts_type() -> String {
        "string".into()
    }

    fn from_json(value: JsonValue) -> Result<Self, PropError> {
        match value {
            JsonValue::String(s) => Ok(s.into()),
            v => Err(PropError::expected("a string", &v)),
        }
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::String(self.as_str().to_owned())
    }
}

impl JsValue for String {
    fn ts_type() -> String {
        "string".into()
    }

    fn from_json(value: JsonValue) -> Result<Self, PropError> {
        match value {
            JsonValue::String(s) => Ok(s),
            v => Err(PropError::expected("a string", &v)),
        }
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::String(self.clone())
    }
}

/// A CSS color string from JS (`"#2563eb"`, `"rgb(0 0 0 / 0.5)"`).
impl JsValue for Color {
    fn ts_type() -> String {
        "string".into()
    }

    fn from_json(value: JsonValue) -> Result<Self, PropError> {
        value
            .as_str()
            .and_then(Color::parse)
            .ok_or_else(|| PropError::expected("a color", &value))
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::String(format!("#{:08x}", self.0))
    }
}

impl JsValue for JsonValue {
    fn ts_type() -> String {
        "unknown".into()
    }

    fn from_json(value: JsonValue) -> Result<Self, PropError> {
        Ok(value)
    }

    fn to_json(&self) -> JsonValue {
        self.clone()
    }
}

/// `null` is `None`.
impl<T: JsValue> JsValue for Option<T> {
    fn ts_type() -> String {
        format!("{} | null", T::ts_type())
    }

    fn from_json(value: JsonValue) -> Result<Self, PropError> {
        match value {
            JsonValue::Null => Ok(None),
            v => T::from_json(v).map(Some),
        }
    }

    fn to_json(&self) -> JsonValue {
        self.as_ref().map_or(JsonValue::Null, T::to_json)
    }
}

impl<T: JsValue> JsValue for Vec<T> {
    fn ts_type() -> String {
        format!("({})[]", T::ts_type())
    }

    fn from_json(value: JsonValue) -> Result<Self, PropError> {
        match value {
            JsonValue::Array(items) => items.into_iter().map(T::from_json).collect(),
            v => Err(PropError::expected("an array", &v)),
        }
    }

    fn to_json(&self) -> JsonValue {
        JsonValue::Array(self.iter().map(T::to_json).collect())
    }
}
