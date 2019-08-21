use std::borrow::Cow;

use crate::distr::{KeyType, Distr};
use crate::operations::Op;
use crate::type_info::Type;
use crate::env::Env;
use crate::error::Error;

#[derive(Clone, Debug)]
pub enum Symbol {
    Nil,
    Text(String),
    Num(KeyType),
    Distr(Distr),
    Seq(Vec<Symbol>),
    Fn(Box<fn(Vec<Symbol>) -> Symbol>, Type),
    Apply{func: Box<Symbol>, exprs: Vec<Symbol>},
    ApplyBuiltin(Vec<Symbol>, Op),
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
            Symbol::Distr(ref d) => Ok(Cow::Owned(d.try_cast()?)),
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
            Symbol::Distr(ref d) => d.stat_view(),
            Symbol::Fn(ref func, ref type_) => format!("<{} at {:?}>", type_, func),
            Symbol::Seq(ref v) => format!("[{}]", v.iter().map(Symbol::repr).collect::<Vec<String>>().join(", ")),
            Symbol::ApplyBuiltin(ref args, op) => op.repr(args),
            Symbol::Apply { ref func, ref exprs } => format!("({} >> {})", exprs.iter().map(Symbol::repr).collect::<Vec<String>>().join(" "), func.repr()),
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
            Symbol::Fn(_, _) => println!("{}{}", indent, self.repr()),
            Symbol::Seq(ref v) => {
                println!("{}Seq: [", indent);
                for symbol in v {
                    symbol.walk(env, indent_level + 4);
                }
                println!("{}]", indent);
            }
            Symbol::ApplyBuiltin(ref args, op) => {
                println!("{}{:?}", indent, op);
                for arg in args {
                    arg.walk(env, indent_level + 4);
                }
            },
            Symbol::Apply {ref func, ref exprs} => {
                println!("{}Apply", indent);
                func.walk(env, indent_level + 4);
                println!("{} to ", indent);
                for exp in exprs {
                    exp.walk(env, indent_level + 4);
                }
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
            Symbol::Fn(_, ref type_) => Ok(type_.clone()),
            Symbol::Seq(ref v) => {
                for symbol in v {
                    let _ = symbol.type_check(env)?;
                }
                //todo type inference that is wicked smaht and can handle zero sized sequences
                Ok(Type::Seq(Box::new(Type::Any)))
            }
            Symbol::Apply {ref func, ref exprs} => {
                // check the type of each input
                let type_args = exprs.iter().map(|arg| arg.type_check(env)).collect::<Result<Vec<Type>, Error>>()?;
                let fn_type = func.type_check(env)?;
                if fn_type.is_any() { return Ok(Type::Any); }
                if let Type::Fn {ref in_types, ref out_type} = func.type_check(env)? {
                    // each type in our argument much be coercible to the corresponding in_type in the signature
                    if in_types.iter().zip(type_args.iter()).all(|(expected, found)| found.coercible_to(expected)) {
                        Ok(*out_type.clone())
                    } else {
                        Err(fail!("function application expected signature {}, not {}", Type::stringify_slice(in_types), Type::stringify_slice(&type_args)))
                    }
                } else {
                    Err(fail!("not a function: {}", func.repr()))
                }
            },
            Symbol::ApplyBuiltin(ref args, op) => {
                let type_args: Result<Vec<Type>, Error> = args.iter().map(|arg| arg.type_check(env)).collect();
                op.type_check(type_args?)
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
            Symbol::Nil | Symbol::Num(_) | Symbol::Distr(_) | Symbol::Fn(_, _) => Cow::Borrowed(self),
            Symbol::Seq(ref v) => {
                // evaluate each item and put it back in a sequence
                Cow::Owned(Symbol::Seq(v.iter().map(|expr| expr.eval(env).map(Cow::into_owned)).collect::<Result<Vec<Symbol>, Error>>()?))
            }
            Symbol::Apply {ref func, ref exprs} => Cow::Owned({
                // evaluate each argument
                let eval_args = exprs.iter().map(|expr| expr.eval(env).map(Cow::into_owned)).collect::<Result<Vec<Symbol>, Error>>()?;
                let eval_func = func.eval(env)?;
                if let Symbol::Fn(ref fn_ptr, ref fn_type) = *eval_func {
                    fn_ptr(eval_args)
                } else {
                    return Err(fail!("not a function: `{}`", eval_func.repr()))
                }
            }),
            Symbol::ApplyBuiltin(args, op) => Cow::Owned({
                let eval_args = args.iter().map(|arg| arg.eval(env)).collect::<Result<Vec<Cow<Symbol>>, Error>>()?;
                op.eval(eval_args)?
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
