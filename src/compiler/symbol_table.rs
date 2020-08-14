use std::collections::HashMap;

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

impl Symbol {
    pub fn is_global(&self) -> bool {
        self.scope == SymbolScope::Global
    }
}

pub struct SymbolTable {
    store: HashMap<String, Symbol>,
    num_definitions: usize,
}

pub struct SymbolTableStack {
    pub stack: Vec<SymbolTable>,
}

impl SymbolTableStack {
    pub fn push(&mut self) {
        self.stack.push(SymbolTable {
            store: HashMap::new(),
            num_definitions: 0,
        })
    }

    pub fn pop(&mut self) -> SymbolTable {
        self.stack.pop().expect("Popped global symbol_table")
    }

    pub fn define(&mut self, name: &str) -> &Symbol {
        let scope = if self.stack.len() == 1 {
            SymbolScope::Global
        } else {
            SymbolScope::Local
        };
        let symbol_table = self.stack.last_mut().expect("There are no symbol_table");

        let symbol = Symbol {
            name: name.to_string(),
            scope,
            index: symbol_table.num_definitions,
        };
        symbol_table.store.insert(name.to_string(), symbol);
        symbol_table.num_definitions += 1;
        symbol_table.store.get(name).unwrap()
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        for symbol_table in self.stack.iter().rev() {
            let result = symbol_table.store.get(name);
            if result.is_some() {
                return result;
            }
        }
        None
    }
}

pub fn new_symbol_table_stack() -> SymbolTableStack {
    let mut stack = SymbolTableStack { stack: vec![] };
    stack.push();
    stack
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

        let mut stack = new_symbol_table_stack();

        // global
        let a = stack.define("a");
        assert_eq!(&expected_a, a);
        let b = stack.define("b");
        assert_eq!(&expected_b, b);

        // first local
        stack.push();
        let c = stack.define("c");
        assert_eq!(&expected_c, c);
        let d = stack.define("d");
        assert_eq!(&expected_d, d);

        // second local
        stack.push();
        let e = stack.define("e");
        assert_eq!(&expected_e, e);
        let f = stack.define("f");
        assert_eq!(&expected_f, f);
    }

    #[test]
    fn test_resolve_global() {
        let mut stack = new_symbol_table_stack();
        stack.define("a");
        stack.define("b");

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
            if let Some(result) = stack.resolve(&sym.name) {
                assert_eq!(sym, result);
            } else {
                assert!(false, format!("name {} not resolvable", sym.name));
            }
        }
    }

    #[test]
    fn test_resolve_local() {
        let mut stack = new_symbol_table_stack();

        // global
        stack.define("a");
        stack.define("b");

        // local
        stack.push();
        stack.define("c");
        stack.define("d");

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
            if let Some(result) = stack.resolve(&sym.name) {
                assert_eq!(sym, result);
            } else {
                assert!(false, format!("name {} not resulved", sym.name))
            }
        }
    }

    #[test]
    fn test_resolve_nested_local() {
        let mut stack = new_symbol_table_stack();

        // global
        stack.define("a");
        stack.define("b");

        // first local
        stack.push();
        stack.define("c");
        stack.define("d");

        // second local
        stack.push();
        stack.define("e");
        stack.define("f");

        // test second local
        {
            let tests = [
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
            ];
            for sym in &tests {
                if let Some(result) = stack.resolve(&sym.name) {
                    assert_eq!(sym, result);
                } else {
                    assert!(false, format!("name {} not resulved", sym.name))
                }
            }
        }

        // test first local
        stack.pop();
        {
            let tests = [
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
            for sym in &tests {
                if let Some(result) = stack.resolve(&sym.name) {
                    assert_eq!(sym, result);
                } else {
                    assert!(false, format!("name {} not resulved", sym.name))
                }
            }
        }
    }
}
