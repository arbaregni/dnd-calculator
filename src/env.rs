use crate::symbols::Symbol;
use crate::type_info::{Type};

use std::collections::HashMap;
use crate::closures::{FnType, FnVal};
use crate::error::Error;

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
    pub fn bind_fn_var(&mut self, name: String, ptr: fn(Vec<Symbol>, &mut Env) -> Result<Symbol, Error>, type_: FnType) -> &mut Env {
        let value = FnVal{ptr, type_: type_.clone(), exprs: vec![]};
        self.bind_var(name, value.into(), type_.into())
    }
    pub fn lookup_var(&self, name: &str) -> Option<(&Symbol, &Type)> {
        self.var_types.get(name)
            .and_then(|type_| self.var_symbols.get(name).map(|symbol| (symbol, type_)))
    }
}