use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    Text(String),
    List(Vec<Value>),
    Object(HashMap<String, Value>),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ValueType {
    Int,
    Float,
    Bool,
    Text,
    Any,
    List(Box<ValueType>),
    Object(HashMap<String, ValueType>),
}

impl ValueType {
    pub fn matches(&self, value: &Value) -> bool {
        match (self, value) {
            (ValueType::Any, _) => true,

            (ValueType::Int, Value::Int(_)) => true,
            (ValueType::Float, Value::Float(_)) => true,
            (ValueType::Bool, Value::Bool(_)) => true,
            (ValueType::Text, Value::Text(_)) => true,

            (ValueType::List(value_type), Value::List(values)) => {
                values.iter().all(|value| value_type.matches(value))
            }

            (ValueType::Object(schema), Value::Object(values)) => {
                schema.iter().all(|(name, value_type)| {
                    values
                        .get(name)
                        .is_some_and(|value| value_type.matches(value))
                })
            }

            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn primitive_types() {
        assert!(ValueType::Int.matches(&Value::Int(1)));
        assert!(ValueType::Float.matches(&Value::Float(1.0)));
        assert!(ValueType::Bool.matches(&Value::Bool(true)));
        assert!(ValueType::Text.matches(&Value::Text("text".into())));

        assert!(!ValueType::Int.matches(&Value::Text("1".into())));
        assert!(!ValueType::Bool.matches(&Value::Int(1)));
    }

    #[test]
    fn any_type() {
        assert!(ValueType::Any.matches(&Value::Int(1)));
        assert!(ValueType::Any.matches(&Value::Float(1.0)));
        assert!(ValueType::Any.matches(&Value::Bool(true)));
        assert!(ValueType::Any.matches(&Value::Text("text".into())));
        assert!(ValueType::Any.matches(&Value::List(vec![])));
        assert!(ValueType::Any.matches(&Value::Object(HashMap::new())));
    }

    #[test]
    fn list() {
        let schema = ValueType::List(Box::new(ValueType::Int));

        assert!(schema.matches(&Value::List(vec![Value::Int(1), Value::Int(2),])));

        assert!(!schema.matches(&Value::List(vec![Value::Int(1), Value::Text("2".into()),])));
    }

    #[test]
    fn object() {
        let schema = ValueType::Object(HashMap::from([
            ("name".into(), ValueType::Text),
            ("age".into(), ValueType::Int),
        ]));

        assert!(schema.matches(&Value::Object(HashMap::from([
            ("name".into(), Value::Text("Vadim".into()),),
            ("age".into(), Value::Int(17),),
        ]))));

        assert!(!schema.matches(&Value::Object(HashMap::from([
            ("name".into(), Value::Text("Vadim".into()),),
            ("age".into(), Value::Text("17".into()),),
        ]))));
    }

    #[test]
    fn missing_field() {
        let schema = ValueType::Object(HashMap::from([("name".into(), ValueType::Text)]));

        assert!(!schema.matches(&Value::Object(HashMap::new())));
    }

    #[test]
    fn nested_values() {
        let schema = ValueType::List(Box::new(ValueType::List(Box::new(ValueType::Int))));

        let value = Value::List(vec![
            Value::List(vec![Value::Int(1), Value::Int(2)]),
            Value::List(vec![Value::Int(3)]),
        ]);

        assert!(schema.matches(&value));
    }

    #[test]
    fn value_uses_native_json() {
        let value = Value::Object(HashMap::from([
            ("username".into(), Value::Text("test".into())),
            ("role_id".into(), Value::Int(10)),
        ]));

        let json = serde_json::to_value(&value).expect("failed to serialize value");

        assert_eq!(
            json,
            serde_json::json!({
                "username": "test",
                "role_id": 10
            })
        );

        let value: Value = serde_json::from_value(json).expect("failed to deserialize value");

        assert!(matches!(value, Value::Object(_)));
    }
}
