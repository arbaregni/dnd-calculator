use crate::symbols::Symbol;
use crate::type_info::Type;

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Env {
    var_bindings: HashMap<String, (Symbol, Type)>,
}
impl Env {
    pub fn new() -> Env {
        Env { var_bindings: HashMap::new() }
    }
    pub fn iter(&self) -> impl Iterator<Item=(&String, &(Symbol, Type))> {
        self.var_bindings.iter()
    }
    pub fn bind_var(&mut self, name: String, value: Symbol, type_: Type) -> &mut Env {
        self.var_bindings.insert(name, (value, type_));
        self
    }
    pub fn lookup_var(&self, name: &str) -> Option<&(Symbol, Type)> {
        self.var_bindings.get(name)
    }
}