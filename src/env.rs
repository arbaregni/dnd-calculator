use crate::symbols::Symbol;
use crate::type_info::Type;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Env {
    var_symbols: HashMap<String, Symbol>,
    var_types: HashMap<String, Type>,
}
impl Env {
    pub fn new() -> Env {
        Env { var_symbols: HashMap::new(), var_types: HashMap::new() }
    }
    pub fn bind_var(&mut self, name: String, value: Symbol, type_: Type) -> &mut Env {
        self.var_symbols.insert(name.clone(), value);
        self.var_types.insert(name, type_);
        self
    }
    pub fn bind_fn_var(&mut self, name: String, value: Symbol) -> &mut Env {
        if let Symbol::Fn(fn_ptr, fn_type) = value {
            self.var_types.insert(name.clone(), fn_type.clone().into());
            self.var_symbols.insert(name, Symbol::Fn(fn_ptr, fn_type));
        } else {
            panic!("bind fn var needed a Symbol::Fn")
        }
        self
    }
    pub fn lookup_var(&self, name: &str) -> Option<(&Symbol, &Type)> {
        self.var_types.get(name)
            .and_then(|type_| self.var_symbols.get(name).map(|symbol| (symbol, type_)))
    }
}