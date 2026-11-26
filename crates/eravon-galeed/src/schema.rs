use eravon_literals::schema::Cell;

use crate::Value;

pub fn value_of(c: Cell) -> Value {
    match c {
        Cell::S(s) => Value::Str(s),
        Cell::N(n) => Value::Int(n),
        Cell::B(b) => Value::Bool(b),
        Cell::A(v) => Value::Arr(v.into_iter().map(value_of).collect()),
        Cell::O(m) => Value::Obj(m.into_iter().map(|(k, v)| (k.to_string(), value_of(v))).collect()),
    }
}

pub fn value_rows(rows: Vec<(&'static str, Cell)>) -> Vec<(String, Value)> {
    eravon_literals::schema::judged(rows).into_iter().map(|(k, v)| (k.to_string(), value_of(v))).collect()
}

pub fn exit_of(_answered: &Value) -> u8 {
    eravon_literals::schema::Exit::Affirmed.code()
}

pub fn answer_bytes(v: &Value) -> Vec<u8> {
    crate::canon::canonical(v).unwrap_or_else(|t| {
        let refusal = Value::Obj(vec![("ok".into(), Value::Bool(false)), ("token".into(), Value::Str(t.name().into()))]);
        crate::canon::canonical(&refusal).unwrap_or_default()
    })
}
