use super::code::{ConcatInstructions, Instructions};
use super::object::hash::HashKey;
use super::object::Object;
use std::collections::HashMap;

pub fn test_instructions(expected: &Vec<Instructions>, actual: &Instructions) {
    let concated = expected.concat();
    assert_eq!(&concated, actual);
}

pub fn test_expected_object<T: Expectable>(expected: &T, actual: &Object) {
    expected.assert_eq(actual);
}

pub enum Expect {
    Integer(i64),
    Instructions(Vec<Instructions>),
}

pub trait Expectable {
    fn assert_eq(&self, actual: &Object);
}

impl Expectable for i64 {
    fn assert_eq(&self, actual: &Object) {
        if let Object::Integer(integer) = actual {
            assert_eq!(self, integer);
        } else {
            assert!(false, "object is not Integer. {}", actual)
        }
    }
}

impl Expectable for bool {
    fn assert_eq(&self, actual: &Object) {
        if let Object::Boolean(boolean) = actual {
            assert_eq!(self, boolean);
        } else {
            assert!(false, "object is not Boolean. {}", actual)
        }
    }
}

impl Expectable for Object {
    fn assert_eq(&self, actual: &Object) {
        match self {
            Object::Null => assert_eq!(self, actual),
            _ => assert!(false, "Not expectable object: {}", self),
        }
    }
}

impl Expectable for &str {
    fn assert_eq(&self, actual: &Object) {
        if let Object::String(string) = actual {
            assert_eq!(self, string);
        } else {
            assert!(false, "object is not String. {}", actual)
        }
    }
}

impl<T: Expectable> Expectable for Vec<T> {
    fn assert_eq(&self, actual: &Object) {
        if let Object::Array(array) = actual {
            assert_eq!(self.len(), array.elements.len());
            for (expected_e, actual_e) in self.iter().zip(array.elements.iter()) {
                expected_e.assert_eq(actual_e);
            }
        } else {
            assert!(false, "object is not Array. {}", actual)
        }
    }
}

impl<T: Expectable> Expectable for HashMap<HashKey, T> {
    fn assert_eq(&self, actual: &Object) {
        if let Object::Hash(hash) = actual {
            assert_eq!(self.len(), hash.pairs.len());
            for (expected_key, expected_value) in self {
                let pair = hash
                    .pairs
                    .get(expected_key)
                    .expect("no pair for given key in Pairs");
                test_expected_object(expected_value, &pair.value);
            }
        } else {
            assert!(false, "object is not Hash. {}", actual)
        }
    }
}

impl Expectable for Vec<Instructions> {
    fn assert_eq(&self, actual: &Object) {
        if let Object::CompiledFunction(func) = actual {
            test_instructions(self, &func.instructions)
        } else {
            assert!(false, "object is not CompiledFunction. {}", actual)
        }
    }
}

impl Expectable for Expect {
    fn assert_eq(&self, actual: &Object) {
        match self {
            Expect::Integer(integer) => integer.assert_eq(actual),
            Expect::Instructions(instructions) => instructions.assert_eq(actual),
        }
    }
}
