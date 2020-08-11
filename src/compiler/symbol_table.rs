use std::cell::Cell;
use std::collections::HashMap;
use typed_arena::Arena;

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

pub struct SymbolTableArena<'a> {
    arena: Arena<SymbolTable<'a>>,
}

#[derive(Debug, PartialEq)]
pub struct SymbolTable<'a> {
    outer: Option<&'a SymbolTable<'a>>,
    store: HashMap<String, Symbol>,
    num_definitions: usize,
}

impl<'a> SymbolTableArena<'a> {
    pub fn new() -> SymbolTableArena<'a> {
        SymbolTableArena {
            arena: Arena::new(),
        }
    }

    pub fn new_symbol_table(&self) -> &mut SymbolTable<'a> {
        self.arena.alloc(SymbolTable {
            outer: None,
            store: HashMap::new(),
            num_definitions: 0,
        })
    }

    pub fn new_enclosed_symbol_table(&self, outer: &'a SymbolTable<'a>) -> &mut SymbolTable<'a> {
        self.arena.alloc(SymbolTable {
            outer: Some(outer),
            store: HashMap::new(),
            num_definitions: 0,
        })
    }
}

impl<'a> SymbolTable<'a> {
    pub fn define(&mut self, name: &str) -> &Symbol {
        let symbol = Symbol {
            name: name.to_string(),
            scope: self
                .outer
                .map_or(SymbolScope::Global, |_| SymbolScope::Local),
            index: self.num_definitions,
        };
        self.store.insert(name.to_string(), symbol);
        self.num_definitions += 1;
        self.store.get(name).unwrap()
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        let result = self.store.get(name);
        if result.is_none() && self.outer.is_some() {
            return self.outer.unwrap().resolve(name);
        }
        return result;
    }
}

pub fn new_symbol_table_arena<'a>() -> SymbolTableArena<'a> {
    SymbolTableArena::new()
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

        let arena = new_symbol_table_arena();

        let global = arena.new_symbol_table();
        let a = global.define("a");
        assert_eq!(&expected_a, a);
        let b = global.define("b");
        assert_eq!(&expected_b, b);

        let first_local = arena.new_enclosed_symbol_table(global);
        let c = first_local.define("c");
        assert_eq!(&expected_c, c);
        let d = first_local.define("d");
        assert_eq!(&expected_d, d);

        let second_local = arena.new_enclosed_symbol_table(first_local);
        let e = second_local.define("e");
        assert_eq!(&expected_e, e);
        let f = second_local.define("f");
        assert_eq!(&expected_f, f);
    }

    #[test]
    fn test_resolve_global() {
        let arena = new_symbol_table_arena();

        let global = arena.new_symbol_table();
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
        let arena = new_symbol_table_arena();

        let global = arena.new_symbol_table();
        global.define("a");
        global.define("b");

        let local = arena.new_enclosed_symbol_table(global);
        local.define("c");
        local.define("d");

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
            if let Some(result) = local.resolve(&sym.name) {
                assert_eq!(sym, result);
            } else {
                assert!(false, format!("name {} not resulved", sym.name))
            }
        }
    }

    #[test]
    fn test_resolve_nested_local() {
        let arena = new_symbol_table_arena();

        let global = arena.new_symbol_table();
        global.define("a");
        global.define("b");

        let first_local = arena.new_enclosed_symbol_table(global);
        first_local.define("c");
        first_local.define("d");

        let second_local = arena.new_enclosed_symbol_table(first_local);
        second_local.define("e");
        second_local.define("f");

        let tests = [
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

        for tt in &tests {
            for sym in &tt.1 {
                if let Some(result) = tt.0.resolve(&sym.name) {
                    assert_eq!(sym, result);
                } else {
                    assert!(false, format!("name {} not resulved", sym.name))
                }
            }
        }
    }
}
