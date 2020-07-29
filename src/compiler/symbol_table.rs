use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Debug, PartialEq)]
enum SymbolScope {
    Global,
    Local,
}

#[derive(Debug, PartialEq)]
pub struct Symbol {
    name: String,
    scope: SymbolScope,
    pub index: usize,
}

pub struct SymbolTable {
    outer: Option<Rc<RefCell<SymbolTable>>>,
    store: HashMap<String, Symbol>,
    num_definitions: usize,
}

impl SymbolTable {
    pub fn define(&mut self, name: &str) -> &Symbol {
        let scope = if self.outer.is_some() {
            SymbolScope::Local
        } else {
            SymbolScope::Global
        };
        let symbol = Symbol {
            name: name.to_string(),
            scope: scope,
            index: self.num_definitions,
        };
        self.store.insert(name.to_string(), symbol);
        self.num_definitions += 1;
        self.store.get(name).unwrap()
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        let result = self.store.get(name);
        if result.is_some() {
            return result;
        }
        if let Some(outer) = &self.outer {
            return outer.borrow().resolve(name);
        }
        None
    }
}

pub fn new_symbol_table<'a>() -> Rc<RefCell<SymbolTable>> {
    Rc::new(RefCell::new(SymbolTable {
        outer: None,
        store: HashMap::new(),
        num_definitions: 0,
    }))
}

pub fn new_enclosed_symbol_table(outer: Rc<RefCell<SymbolTable>>) -> Rc<RefCell<SymbolTable>> {
    Rc::new(RefCell::new(SymbolTable {
        outer: Some(outer),
        store: HashMap::new(),
        num_definitions: 0,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_define() {
        let expected_a = Symbol {
            name: "a".to_string(),
            scope: SymbolScope::Global,
            index: 0,
        };
        let expected_b = Symbol {
            name: "b".to_string(),
            scope: SymbolScope::Global,
            index: 1,
        };
        let expected_c = Symbol {
            name: "c".to_string(),
            scope: SymbolScope::Local,
            index: 0,
        };
        let expected_d = Symbol {
            name: "d".to_string(),
            scope: SymbolScope::Local,
            index: 1,
        };
        let expected_e = Symbol {
            name: "e".to_string(),
            scope: SymbolScope::Local,
            index: 0,
        };
        let expected_f = Symbol {
            name: "f".to_string(),
            scope: SymbolScope::Local,
            index: 1,
        };

        let global = new_symbol_table();

        let mut global_borrow = global.borrow_mut();
        let a = global_borrow.define("a");
        assert_eq!(&expected_a, a);
        let b = global_borrow.define("b");
        assert_eq!(&expected_b, b);

        let first_local = new_enclosed_symbol_table(Rc::clone(&global));

        let mut first_local_borrow = first_local.borrow_mut();
        let c = first_local_borrow.define("c");
        assert_eq!(&expected_c, c);
        let d = first_local_borrow.define("d");
        assert_eq!(&expected_d, d);

        let second_local = new_enclosed_symbol_table(Rc::clone(&first_local));

        let mut second_local_borrow = second_local.borrow_mut();
        let e = second_local_borrow.define("e");
        assert_eq!(&expected_e, e);
        let f = second_local_borrow.define("f");
        assert_eq!(&expected_f, f);
    }

    #[test]
    fn test_resolve_global() {
        let mut global = new_symbol_table();
        global.borrow_mut().define("a");
        global.borrow_mut().define("b");

        let expected = [
            Symbol {
                name: "a".to_string(),
                scope: SymbolScope::Global,
                index: 0,
            },
            Symbol {
                name: "b".to_string(),
                scope: SymbolScope::Global,
                index: 1,
            },
        ];

        for sym in &expected {
            if let Some(result) = global.borrow().resolve(&sym.name) {
                assert_eq!(sym, result);
            } else {
                assert!(false, format!("name {} not resolvable", sym.name));
            }
        }
    }

    #[test]
    fn test_resolve_local() {
        let mut global = new_symbol_table();
        global.borrow_mut().define("a");
        global.borrow_mut().define("b");

        let mut local = new_enclosed_symbol_table(global);
        local.borrow_mut().define("c");
        local.borrow_mut().define("d");

        let expected = vec![
            Symbol {
                name: "a".to_string(),
                scope: SymbolScope::Global,
                index: 0,
            },
            Symbol {
                name: "b".to_string(),
                scope: SymbolScope::Global,
                index: 1,
            },
            Symbol {
                name: "c".to_string(),
                scope: SymbolScope::Local,
                index: 0,
            },
            Symbol {
                name: "d".to_string(),
                scope: SymbolScope::Local,
                index: 1,
            },
        ];

        for sym in &expected {
            if let Some(result) = local.borrow().resolve(&sym.name) {
                assert_eq!(sym, result);
            } else {
                assert!(false, format!("name {} not resulved", sym.name))
            }
        }
    }

    #[test]
    fn test_resolve_nested_local() {
        let global = new_symbol_table();
        let mut global_borrow = global.borrow_mut();
        global_borrow.define("a");
        global_borrow.define("b");

        let first_local = new_enclosed_symbol_table(Rc::clone(&global));
        let mut first_local_borrow = first_local.borrow_mut();
        first_local_borrow.define("c");
        first_local_borrow.define("d");

        let second_local = new_enclosed_symbol_table(Rc::clone(&first_local));
        let mut second_local_borrow = second_local.borrow_mut();
        second_local_borrow.define("e");
        second_local_borrow.define("f");

        let tests = [
            (
                Rc::clone(&first_local),
                [
                    Symbol {
                        name: "a".to_string(),
                        scope: SymbolScope::Global,
                        index: 0,
                    },
                    Symbol {
                        name: "b".to_string(),
                        scope: SymbolScope::Global,
                        index: 1,
                    },
                    Symbol {
                        name: "c".to_string(),
                        scope: SymbolScope::Local,
                        index: 0,
                    },
                    Symbol {
                        name: "d".to_string(),
                        scope: SymbolScope::Local,
                        index: 1,
                    },
                ],
            ),
            (
                Rc::clone(&second_local),
                [
                    Symbol {
                        name: "a".to_string(),
                        scope: SymbolScope::Global,
                        index: 0,
                    },
                    Symbol {
                        name: "b".to_string(),
                        scope: SymbolScope::Global,
                        index: 1,
                    },
                    Symbol {
                        name: "e".to_string(),
                        scope: SymbolScope::Local,
                        index: 0,
                    },
                    Symbol {
                        name: "f".to_string(),
                        scope: SymbolScope::Local,
                        index: 1,
                    },
                ],
            ),
        ];

        for tt in &tests {
            for sym in &tt.1 {
                if let Some(result) = tt.0.borrow().resolve(&sym.name) {
                    assert_eq!(sym, result);
                } else {
                    assert!(false, format!("name {} not resulved", sym.name))
                }
            }
        }
    }
}
