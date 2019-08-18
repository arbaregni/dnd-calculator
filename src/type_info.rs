#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
    Nil,
    Any,
    Num,
    Distr,
    Seq(Box<Type>),
    Fn{in_types: Vec<Type>, out_type: Box<Type>}
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match *self {
            Type::Nil => write!(f, "Nil"),
            Type::Any => write!(f, "Any"),
            Type::Num => write!(f, "Num"),
            Type::Distr => write!(f, "Distr"),
            Type::Seq(ref inner_type) => write!(f, "Seq<{}>", inner_type),
            Type::Fn{ref in_types, ref out_type} => {
                write!(f, "Fn({}) -> {}", Type::stringify_slice(in_types), out_type)
            }
        }
    }
}
impl Type {
    pub fn try_from(s: &str) -> Option<Type> {
        match s {
            "Nil" => Some(Type::Nil),
            "Num" => Some(Type::Num),
            "Distr" => Some(Type::Distr),
            "Any" => Some(Type::Any),
            _ => None
        }
    }
    pub fn stringify_slice(slice: &[Type]) -> String {
        let mut s = String::new();
        for type_ in slice {
            s.push_str(&format!("{}", type_));
        }
        s
    }
    pub fn is_any(&self) -> bool { *self == Type::Any }
    pub fn coercible_to(&self, type_: &Type) -> bool {
        // Type::Any can be coerced into anything and anything can be coerced into Type::Any
        if *self == Type::Any || *type_ == Type::Any {
            return true;
        }
        // numbers can be coerced into distributions
        if *self == Type::Num && *type_ == Type::Distr {
            return true;
        }
        // functions are coerced based on their output

        // otherwise they must match exactly
        *self == *type_
    }
}