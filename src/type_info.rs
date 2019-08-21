#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FnType {
    pub in_types: Vec<Type>,
    pub out_type: Box<Type>,
}
/// create an FnType from the macro invocation in the form
///     `fn_type!(Distr, Distr, -> Distr)`
/// This yields `FnType(vec![Type::Distr, Type::Distr], Type::Distr)`
/// the out_type can be any expression, but the in_types must be paths
/// The terminating comma is required due to the restrictions on capturing paths
/// `let` statements must be used for complex types
/// ```
/// let distr_seq = Type::Seq(Box::new(Type::Distr);
/// assert_eq!(fn_type!(distr_seq, Type::Distr -> Type::Distr), FnType{in_types: vec![Seq(Box::new(Distr), Distr], out_type: Box::new(Distr)})
/// ```
macro_rules! fn_type {
    ($($inp:path,)* -> $out_type:expr) => {
        FnType{in_types: vec![$($inp),*], out_type: Box::new($out_type)}
    }
}
impl std::fmt::Display for FnType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        let in_types = self.in_types.iter().map(|type_| format!("{}", type_)).collect::<Vec<String>>().join(", ");
        write!(f, "Fn({}) -> {}", in_types, self.out_type)
    }
}
impl std::convert::From<FnType> for Type {
    fn from(fn_type: FnType) -> Self {
        Type::Fn(fn_type)
    }
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Type {
    Nil,
    Any,
    Num,
    Distr,
    Seq(Box<Type>),
    Fn(FnType),
}

impl std::fmt::Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match *self {
            Type::Nil => write!(f, "Nil"),
            Type::Any => write!(f, "Any"),
            Type::Num => write!(f, "Num"),
            Type::Distr => write!(f, "Distr"),
            Type::Seq(ref inner_type) => write!(f, "Seq<{}>", inner_type),
            Type::Fn(ref fn_type) => write!(f, "{}", fn_type),
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
    pub fn try_to_fn(&self) -> Option<&FnType> {
        if let Type::Fn(ref fn_type) = *self {
            Some(fn_type)
        } else {
            None
        }
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