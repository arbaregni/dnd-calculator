use crate::type_info::Type;
use crate::symbols::Symbol;
use std::borrow::Cow;
use std::cmp::Ordering;
use crate::error::Error;
use crate::env::Env;
use std::fmt::Formatter;

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

/// # Fields
/// * `ptr` - boxed pointer to the underlying function to evoke
/// * `type_` - FnType representing the input and output types of this function
/// * `exprs` - Vec of Symbols representing the already applied (captured) arguments
///
/// # Invariant
/// the len of `type_.in_types` and the len of `exprs` should always equal the number of arguments that the underlying function pointer expects
///
/// # Example
/// ```
/// let ptr = Box::new(|vec| vec[2]);
/// let type_ = fn_type!(Type::Num, Type::Distr, Type::Nil, -> Type::Num);
/// let fn_symbol = Symbol::Fn{ptr, type_, exprs: vec![]};
/// // at this point, the underlying pointer expects a vector of len 3
/// // we have not applied any inputs yet, so type_.input_types contains all the inputs
/// let apply_symbol = Symbol::Apply{target: fn_symbol, args: vec![Symbol::Num(1)]};
/// // -- snip --
/// // apply_symbol is evaluated
/// // -- snip --
/// // let expected_result = Symbol::Fn{ptr, type_: fn_type!(Type::Distr, Type::Nil), exprs: vec![Symbol::Num(1)]};
///
/// ```
#[derive(Clone)]
pub struct FnVal {
    pub ptr: fn(Vec<Symbol>, &mut Env) -> Result<Symbol, Error>,
    pub type_: FnType,
    pub exprs: Vec<Symbol>,
}
impl FnVal {
    pub fn repr(&self) -> String { format!("{:?}", self) }
    pub fn apply<'s>(&'s self, args: &[Symbol], env: &mut Env) -> Result<Symbol, Error> {
        let mut new_exprs: Vec<Symbol> = vec![];
        new_exprs.extend_from_slice(self.exprs.as_slice()); // these were applied previously
        new_exprs.extend_from_slice(args); // we are applying those now
        match args.len().cmp(&self.type_.in_types.len()) {
            Ordering::Less => {
                // more to go: wrap up what we have in a Symbol::Fn
                Ok(FnVal{ ptr: self.ptr, type_: self.type_.curry(args.len()), exprs: new_exprs}.into())
            },
            Ordering::Equal => {
                // we are done: time to evaluate!
                let evaluated = new_exprs.iter().map(|expr| expr.eval(env).map(Cow::into_owned)).collect::<Result<Vec<Symbol>, Error>>()?;
                (self.ptr)(evaluated, env)
            },
            Ordering::Greater => {
                // we went to far: let's complain >:(
                // todo make this error more helpful
                return Err(fail!("function {} was applied too many arguments (expected {} more, was given {})", self.repr(), self.type_.in_types.len(),  args.len()));
            },
        }
    }
}
impl std::fmt::Debug for FnVal {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "<{} at {:?}>", self.type_, self.ptr as *const usize)
    }
}
impl std::convert::From<FnVal> for Symbol {
    fn from(fn_val: FnVal) -> Self {
        Symbol::Fn(fn_val)
    }
}