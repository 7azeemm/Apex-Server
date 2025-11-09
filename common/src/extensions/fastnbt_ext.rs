use fastnbt::Value;
use std::collections::HashMap;

pub trait ValueExt {
    fn as_compound(&self) -> Option<&HashMap<String, Value>>;
    fn as_list(&self) -> Option<&Vec<Value>>;
}

impl ValueExt for Value {
    fn as_compound(&self) -> Option<&HashMap<String, Value>> {
        match self {
            Value::Compound(v) => Some(v),
            _ => None,
        }
    }

    fn as_list(&self) -> Option<&Vec<Value>> {
        match self {
            Value::List(v) => Some(v),
            _ => None,
        }
    }
}
