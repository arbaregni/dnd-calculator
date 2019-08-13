#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
    Nil,
    Num,
    Distr,
    Fn{in_types: Vec<Type>, out_type: Box<Type>}
}

impl Type {
    pub fn try_from(s: &str) -> Option<Type> {
        match s {
            "Nil" => Some(Type::Nil),
            "Num" => Some(Type::Num),
            "Distr" => Some(Type::Distr),
            _ => None
        }
    }
    pub fn stringify_slice(slice: &[Type]) -> String {
        let mut s = String::new();
        for typ in slice {
            s.push_str(&typ.to_string())
        }
        s
    }
    pub fn to_string(&self) -> String {
        match *self {
            Type::Nil => "Nil".to_string(),
            Type::Num => "Num".to_string(),
            Type::Distr => "Distr".to_string(),
            Type::Fn{ref in_types, ref out_type} => {
                format!("Fn({}) -> {}", Type::stringify_slice(in_types), out_type.to_string())
            }
        }
    }
    pub fn coercible_to(&self, type_: &Type) -> bool {
        // the only coercion is from a number to a distribution,
        if *self == Type::Num && *type_ == Type::Distr {
            return true;
        }
        // otherwise they must match exactly
        *self == *type_
    }
}