use super::object::Object;

pub fn test_expected_object<T: Expectable>(expected: &T, actual: &Object) {
  expected.assert_eq(actual);
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
