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

#[derive(Debug, PartialEq)]
pub struct SymbolTable<'a> {
    pub outer: Option<&'a SymbolTable<'a>>,
    store: HashMap<String, Symbol>,
    num_definitions: usize,
}

impl<'a> SymbolTable<'a> {
    pub fn define(&mut self, name: &str) -> &Symbol {
        let scope = match self.outer {
            Some(_) => SymbolScope::Local,
            None => SymbolScope::Global,
        };
        let symbol = Symbol {
            name: name.to_string(),
            scope,
            index: self.num_definitions,
        };
        self.store.insert(name.to_string(), symbol);
        self.num_definitions += 1;
        self.store.get(name).unwrap()
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        // match self.store.get(name) {
        //     Some(obj) => Some(obj),
        //     None => {
        //         if let Some(outer) = self.outer {
        //             outer.resolve(name)
        //         } else {
        //             None
        //         }
        //     }
        // }
        self.store
            .get(name)
            .or_else(|| self.outer.and_then(|outer| outer.resolve(name)))
    }
}

pub fn new_symbol_table<'a>() -> SymbolTable<'a> {
    SymbolTable {
        outer: None,
        store: HashMap::new(),
        num_definitions: 0,
    }
}

pub fn new_enclosed_symbol_table<'a>(outer: &'a SymbolTable) -> SymbolTable<'a> {
    SymbolTable {
        outer: Some(outer),
        store: HashMap::new(),
        num_definitions: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_define() {
        let mut expected = HashMap::new();
        expected.insert(
            "a",
            Symbol {
                name: "a".to_string(),
                scope: SymbolScope::Global,
                index: 0,
            },
        );
        expected.insert(
            "b",
            Symbol {
                name: "b".to_string(),
                scope: SymbolScope::Global,
                index: 1,
            },
        );
        expected.insert(
            "c",
            Symbol {
                name: "c".to_string(),
                scope: SymbolScope::Local,
                index: 0,
            },
        );
        expected.insert(
            "d",
            Symbol {
                name: "d".to_string(),
                scope: SymbolScope::Local,
                index: 1,
            },
        );
        expected.insert(
            "e",
            Symbol {
                name: "e".to_string(),
                scope: SymbolScope::Local,
                index: 0,
            },
        );
        expected.insert(
            "f",
            Symbol {
                name: "f".to_string(),
                scope: SymbolScope::Local,
                index: 1,
            },
        );

        let mut global = new_symbol_table();

        let a = global.define("a");
        assert_eq!(&expected["a"], a);
        let b = global.define("b");
        assert_eq!(&expected["b"], b);

        let mut first_local = new_enclosed_symbol_table(&global);
        let c = first_local.define("c");
        assert_eq!(&expected["c"], c);
        let d = first_local.define("d");
        assert_eq!(&expected["d"], d);

        let mut second_local = new_enclosed_symbol_table(&first_local);
        let e = second_local.define("e");
        assert_eq!(&expected["e"], e);
        let f = second_local.define("f");
        assert_eq!(&expected["f"], f);
    }

    #[test]
    fn test_resolve_global() {
        let mut global = new_symbol_table();
        global.define("a");
        global.define("b");

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
            if let Some(result) = global.resolve(&sym.name) {
                assert_eq!(sym, result);
            } else {
                assert!(false, format!("name {} not resolvable", sym.name));
            }
        }
    }

    #[test]
    fn test_resolve_local() {
        let mut global = new_symbol_table();
        global.define("a");
        global.define("b");

        let mut local = new_enclosed_symbol_table(&global);
        local.define("c");
        local.define("d");

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
            let result = local
                .resolve(&sym.name)
                .expect(&format!("name {} not resolvable", sym.name));
            assert_eq!(result, sym);
        }
    }

    #[test]
    fn test_resolve_nested_local() {
        let mut global = new_symbol_table();
        global.define("a");
        global.define("b");

        let mut first_local = new_enclosed_symbol_table(&global);
        first_local.define("c");
        first_local.define("d");

        let mut second_local = new_enclosed_symbol_table(&first_local);
        second_local.define("e");
        second_local.define("f");

        let tests = vec![
            (
                &first_local,
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
                &second_local,
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

        for tt in tests {
            for sym in &tt.1 {
                let result =
                    tt.0.resolve(&sym.name)
                        .expect(&format!("name {} not resolvable", sym.name));
                assert_eq!(result, sym);
            }
        }
    }
}
