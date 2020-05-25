use std::collections::HashMap;

#[derive(Debug, PartialEq)]
enum SymbolScope {
    Global,
}

#[derive(Debug, PartialEq)]
pub struct Symbol {
    name: String,
    scope: SymbolScope,
    pub index: usize,
}

pub struct SymbolTable {
    store: HashMap<String, Symbol>,
    num_definitions: usize,
}

impl SymbolTable {
    pub fn define(&mut self, name: &str) -> &Symbol {
        let symbol = Symbol {
            name: name.to_string(),
            scope: SymbolScope::Global,
            index: self.num_definitions,
        };
        self.store.insert(name.to_string(), symbol);
        self.num_definitions += 1;
        self.store.get(name).unwrap()
    }

    pub fn resolve(&self, name: &str) -> Option<&Symbol> {
        self.store.get(name)
    }
}

pub fn new_symbol_table() -> SymbolTable {
    SymbolTable {
        store: HashMap::new(),
        num_definitions: 0,
    }
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

        let mut global = new_symbol_table();

        let a = global.define("a");
        assert_eq!(&expected_a, a);
        let b = global.define("b");
        assert_eq!(&expected_b, b);
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
}
