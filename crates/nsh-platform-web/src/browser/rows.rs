//! Keyed row matching stays in Rust; the browser supplies row closures.
use std::{cell::RefCell, collections::HashMap};

use nsh_core::{element::PropError, js::host::HostInput};

use super::values::{Value, invoke};
#[derive(Hash, PartialEq, Eq)]
enum Key {
    Object(u32),
    String(String),
    Number(u64),
    Primitive(u32),
}
impl Key {
    fn of(value: &Value) -> Result<Self, PropError> {
        Ok(match value.kind() {
            3 => Self::Number(value.as_number().unwrap_or(f64::NAN).to_bits()),
            4 => Self::String(value.as_string().unwrap_or_default()),
            5..=7 => Self::Object(invoke(33, value.0, 0, 0, 0.0) as u32),
            0..=2 => Self::Primitive(value.0),
            _ => {
                return Err(PropError(
                    "symbol and bigint row keys need an identity handle".into(),
                ));
            }
        })
    }
}
struct Row {
    key: Key,
    _key_value: Value,
    value: Value,
}
#[derive(Default)]
pub(super) struct Rows {
    lists: RefCell<Vec<Option<Vec<Row>>>>,
}
impl Rows {
    fn dispose(row: Row) {
        if let Ok(dispose) = row.value.get("dispose") {
            let _ = Value::array([]).and_then(|args| dispose.call(&Value(0), &args));
        }
    }
    pub(super) fn call(&self, op: u32, args: &Value) -> Result<Value, PropError> {
        if op == 310 {
            let mut lists = self.lists.borrow_mut();
            let id = lists.len();
            lists.push(Some(Vec::new()));
            return Ok(Value::number(id as f64));
        }
        let id = args
            .index(0)?
            .as_number()
            .ok_or_else(|| PropError("invalid row list".into()))? as usize;
        let old = self
            .lists
            .borrow_mut()
            .get_mut(id)
            .and_then(Option::take)
            .ok_or_else(|| PropError("row list disposed or updating".into()))?;
        if op == 312 {
            for row in old {
                Self::dispose(row);
            }
            return Ok(Value(0));
        }
        let mut old: Vec<_> = old.into_iter().map(Some).collect();
        let mut keys: HashMap<Key, Vec<usize>> = HashMap::new();
        // Key values remain retained, so object identities cannot disappear.
        for (index, row) in old.iter().enumerate().rev() {
            let row = row.as_ref().unwrap();
            keys.entry(Key::of(&row._key_value)?)
                .or_default()
                .push(index);
        }
        let mut next = Vec::new();
        let result = (|| {
            let items = args.index(1)?;
            let factory = args.index(2)?;
            let key_fn = args.index(3)?;
            let length = items.length()?.unwrap_or(0);
            if length > 1_000_000 {
                return Err(PropError("row count exceeds traversal limit".into()));
            }
            for index in 0..length {
                let item = items.index(index)?;
                let position = Value::number(index as f64);
                let key_value = if key_fn.kind() == 6 {
                    key_fn.call(&Value(0), &Value::array([item.clone(), position.clone()])?)?
                } else if key_fn.as_string().as_deref() == Some("index") {
                    position.clone()
                } else {
                    item.clone()
                };
                let key = Key::of(&key_value)?;
                if let Some(row) = keys
                    .get_mut(&key)
                    .and_then(Vec::pop)
                    .and_then(|index| old[index].take())
                {
                    let result = (|| {
                        for (name, value) in [("item", item), ("index", position)] {
                            let signal = row.value.get(name)?;
                            signal
                                .get("set")?
                                .call(&Value(0), &Value::array([value])?)?;
                        }
                        Ok::<_, PropError>(())
                    })();
                    next.push(row);
                    result?;
                } else {
                    let value = factory.call(&Value(0), &Value::array([item, position])?)?;
                    next.push(Row {
                        key,
                        _key_value: key_value,
                        value,
                    });
                }
            }
            Value::array(
                next.iter()
                    .map(|row| row.value.get("output"))
                    .collect::<Result<Vec<_>, _>>()?,
            )
        })();
        for row in old.into_iter().flatten() {
            Self::dispose(row);
        }
        self.lists.borrow_mut()[id] = Some(next);
        result
    }
}
