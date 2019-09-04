use crate::type_info::Type;

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
impl FnType {
    /// produce a new FnType
    /// `num`: the number of inputs to be curried
    /// ```
    /// let original = fn_type!(Type::Distr, Type::Nil, -> Type::Any);
    /// let curried = original.curry(1);
    /// assert_eq!(fn_type!(Type::Nil, -> Type::Any), curried)
    /// ```
    pub fn curry(&self, num: usize) -> FnType {
        FnType { in_types: self.in_types[num..].to_vec(), out_type: self.out_type.clone() }
    }
}
