use std::borrow::Cow;

use crate::distr::{KeyType, Distr};
use crate::type_info::{Type, FnType};
use crate::env::Env;
use crate::error::Error;
use std::cmp::Ordering;

#[derive(Clone, Debug)]
pub enum Symbol {
    Nil,
    Text(String),
    Num(KeyType),
    Distr(Distr),
    Seq(Vec<Symbol>),
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
    Fn{ptr: Box<fn(Vec<Symbol>) -> Symbol>, type_: FnType, exprs: Vec<Symbol>},
    Apply{target: Box<Symbol>, args: Vec<Symbol>},
    Assigner{name: String, def_type: Option<Type>, expr: Box<Symbol>},
}

impl Symbol {
    pub fn into_boxed(self) -> Box<Symbol> {
        Box::new(self)
    }
    pub fn try_to_distr(&self) -> Result<Cow<Distr>, Error> {
        match *self {
            Symbol::Distr(ref d) => Ok(Cow::Borrowed(d)),
            Symbol::Num(num) => Ok(Cow::Owned(num.into())),
            _ => Err(fail!("{} is not a distr", self.repr())),
        }
    }
    pub fn try_to_num(&self) -> Result<Cow<KeyType>, Error> {
        match *self {
            Symbol::Num(num) => Ok(Cow::Owned(num)),
            Symbol::Distr(ref d) => Ok(Cow::Owned(d.try_to_num()?)),
            _ => Err(fail!("{} is not a number", self.repr())),
        }
    }
    pub fn try_to_str(&self) -> Result<&str, Error> {
        match *self {
            Symbol::Text(ref s) => Ok(s),
            _ => Err(fail!("{} is not a string", self.repr()))
        }
    }
    pub fn repr(&self) -> String {
        match *self {
            Symbol::Nil => format!("Nil"),
            Symbol::Text(ref s) => format!("{}", s),
            Symbol::Num(n) => format!("{}", n),
            Symbol::Distr(ref d) => d.try_to_num().map(|n| format!("{}", n)).unwrap_or(d.stat_view()),
            Symbol::Fn{ref ptr, ref type_, ref exprs} => format!("<{} at {:?}>", type_, ptr),
            Symbol::Seq(ref v) => format!("[{}]", v.iter().map(Symbol::repr).collect::<Vec<String>>().join(", ")),
            Symbol::Apply { ref target, ref args } => format!("({} >> {})", args.iter().map(Symbol::repr).collect::<Vec<String>>().join(" "), target.repr()),
            Symbol::Assigner { ref name, ref def_type, ref expr } => {
                match def_type {
                    None => format!("{} = {}", name, expr.repr()),
                    Some(type_) => format!("{}: {} = {}", name, type_, expr.repr()),
                }
            },
        }
    }
    pub fn walk(&self, env: &Env, indent_level: usize) {
        let indent: &String = &(0..indent_level).map(|_| ' ').collect();
        match *self {
            Symbol::Nil => println!("{}Nil", indent),
            Symbol::Text(ref text) => {
                println!("{}Text {} := ", indent, text);
                if let Some((ref symbol, _)) = env.lookup_var(text) {
                    symbol.walk(env, indent_level + 4);
                }
            },
            Symbol::Num(num) => println!("{}Num: {}", indent, num),
            Symbol::Distr(ref distr) => println!("{}Distr{}", indent, distr.stat_view()),
            Symbol::Fn { ref exprs , .. } => {
                println!("{}{}, captured: ", indent, self.repr());
                println!("{}[", indent);
                for expr in exprs {
                    expr.walk(env, indent_level + 4);
                }
                println!("{}]", indent);
            },
            Symbol::Seq(ref v) => {
                println!("{}Seq: [", indent);
                for symbol in v {
                    symbol.walk(env, indent_level + 4);
                }
                println!("{}]", indent);
            }
            Symbol::Apply {ref target, ref args} => {
                println!("{}Apply", indent);
                for arg in args {
                    arg.walk(env, indent_level + 4);
                }
                println!("{} to ", indent);
                target.walk(env, indent_level + 4);
            },
            Symbol::Assigner {ref name, ref def_type, ref expr} => {
                println!("{}Assigner[{}: {:?}]", indent, name, def_type);
                expr.walk(env, indent_level + 4);
            }
        }
    }
    pub fn type_check(&self, env: &Env) -> Result<Type, Error> {
        match *self {
            Symbol::Nil => Ok(Type::Nil),
            Symbol::Num(_) => Ok(Type::Num),
            Symbol::Distr(_) => Ok(Type::Distr),
            Symbol::Fn { ref type_, .. } => Ok(type_.clone().into()),
            Symbol::Seq(ref v) => {
                for symbol in v {
                    let _ = symbol.type_check(env)?;
                }
                //todo type inference that is wicked smaht and can handle zero sized sequences
                Ok(Type::Seq(Box::new(Type::Any)))
            }
            Symbol::Apply {ref target, ref args} => {
                let type_ = target.type_check(env)?;
                if type_.is_any() { return Ok(Type::Any); } // Any type skips type checking until evaluation
                if let Type::Fn(fn_type) = type_ {
                    if args.len() > fn_type.in_types.len() {
                        return Err(fail!("too many arguments applied to function ({} expected {}, gave it {})", target.repr(), fn_type.in_types.len(), args.len()))
                    }
                    // each type in our argument much be coercible to the corresponding in_type in the signature
                    for (i, (arg, expected_type)) in args.iter().zip(fn_type.in_types.iter()).enumerate() {
                        let found_type = arg.type_check(env)?;
                        if !found_type.coercible_to(expected_type) {
                            return Err(fail!("incorrect signature for function at position {}: expected type {}, {}", i, expected_type, found_type))
                        }
                    }
                    if args.len() < fn_type.in_types.len() {
                        // more to do: the function will be curried
                        Ok(fn_type.curry(args.len()).into())
                    } else {
                        // exact match: the underlying function pointer will be evoked
                        Ok(fn_type.out_type.as_ref().clone())
                    }
                } else {
                    return Err(fail!("not a function: {}, found type {}", target.repr(), type_));
                }
            },
            Symbol::Text(ref name) => {
                if let Some((_, type_)) = env.lookup_var(name) {
                    // ignore the symbol: may be a placeholder
                    Ok(type_.clone())
                } else {
                    Err(fail!("{:?} has no binding in current namespace", name))
                }
            }
            Symbol::Assigner {name: _, ref def_type, ref expr} => {
                //TODO typecheck with arguments ??
                let concrete_type = expr.type_check(env)?;
                if let Some(_type) = def_type {
                    if !concrete_type.coercible_to(_type) {
                        return Err(fail!("annotated type {:?} does not match concrete type {:?}", _type, concrete_type));
                    }
                }
                Ok(Type::Nil)
            }
        }
    }
    pub fn eval(&self, env: &mut Env) -> Result<Cow<Symbol>, Error> {
        Ok(match self {
            Symbol::Nil | Symbol::Num(_) | Symbol::Distr(_) | Symbol::Fn{..}=> Cow::Borrowed(self),
            Symbol::Seq(ref v) => {
                // evaluate each item and put it back in a sequence
                Cow::Owned(Symbol::Seq(v.iter().map(|expr| expr.eval(env).map(Cow::into_owned)).collect::<Result<Vec<Symbol>, Error>>()?))
            }
            Symbol::Apply {ref target, ref args} => Cow::Owned({
                let eval_func = target.eval(env)?;
                if let Symbol::Fn { ref ptr, ref type_, ref exprs} = eval_func.as_ref() {
                    let mut new_exprs: Vec<Symbol> = vec![];
                    new_exprs.extend_from_slice(exprs.as_slice()); // these were applied previously
                    new_exprs.extend_from_slice(args.as_slice()); // we are applying those now
                    println!("new_exprs: {:?}", new_exprs);
                    match args.len().cmp(&type_.in_types.len()) {
                        Ordering::Less => {
                            // more to go: wrap up what we have in a Symbol::Fn
                            Symbol::Fn { ptr: Box::clone(ptr), type_: type_.curry(args.len()), exprs: new_exprs}
                        },
                        Ordering::Equal => {
                            // we are done: time to evaluate!
                            let evaluated = new_exprs.iter().map(|expr| expr.eval(env).map(Cow::into_owned)).collect::<Result<Vec<Symbol>, Error>>()?;
                            println!("evaluated: {:#?}", evaluated);
                            ptr(evaluated)
                        },
                        Ordering::Greater => {
                            // we went to far: let's complain >:(
                            // todo make this error more helpful
                            return Err(fail!("function {} was applied too many arguments (expected {} more, was given {})", target.repr(), type_.in_types.len(),  args.len()));
                        },
                    }
                } else {
                    return Err(fail!("not a function: {}", eval_func.repr()))
                }
            }),
            Symbol::Text(ref name) => {
                if let Some(value) = env.lookup_var(name).map(|(x, _)| x.clone()) {
                    Cow::Owned(value)
                } else {
                    Cow::Borrowed(self)
                }
            },
            Symbol::Assigner {ref name, def_type: _, ref expr} => {
                let value = expr.eval(env)?;
                let type_ = value.type_check(env)?;
                env.bind_var(name.clone(), value.into_owned(), type_);
                Cow::Owned(Symbol::Nil)
            }
        })
    }
}
impl std::convert::From<KeyType> for Symbol {
    fn from(n: KeyType) -> Symbol {
        Symbol::Num(n)
    }
}
impl std::convert::From<Distr> for Symbol {
    fn from(distr: Distr) -> Symbol {
        Symbol::Distr(distr)
    }
}
impl std::convert::From<String> for Symbol {
    fn from(s: String) -> Symbol {
        Symbol::Text(s)
    }
}
impl std::convert::From<()> for Symbol {
    fn from(_: ()) -> Symbol {
        Symbol::Nil
    }
}
