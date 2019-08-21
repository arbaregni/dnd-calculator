use crate::type_info::Type;
use crate::symbols::Symbol;
use crate::error::Error;

use std::borrow::Cow;

#[derive(Debug, Copy, Clone)]
pub enum Op {
    Mul,
    Div,
    Add,
    Sub,
    StatView,
    HistView,
    TableView,
    MakeDice,
    MakeDiceSingle,
}

impl Op {
    pub fn repr(&self, args: &Vec<Symbol>) -> String {
        match *self {
            Op::Mul => format!("({} * {})", args[0].repr(), args[1].repr()),
            Op::Div => format!("({} / {})", args[0].repr(), args[1].repr()),
            Op::Add => format!("({} + {})", args[0].repr(), args[1].repr()),
            Op::Sub => format!("({} - {})", args[0].repr(), args[1].repr()),
            Op::StatView => format!("stats {}", args[0].repr()),
            Op::HistView => format!("hist {}", args[0].repr()),
            Op::TableView => format!("table {}", args[0].repr()),
            Op::MakeDice => format!("{}d{}", args[0].repr(), args[1].repr()),
            Op::MakeDiceSingle => format!("d{}", args[0].repr()),
        }
    }
    pub fn type_check(&self, arg_types: Vec<Type>) -> Result<Type, Error> {
        macro_rules! check_arg {
            ($idx:expr, $typ:expr) => (
                if !arg_types[$idx].coercible_to($typ) {
                    return Err(fail!("operation {:?} expected arg in position {} to be type {}, not {}", self, $idx, $typ, arg_types[$idx]));
                }
            )
        }
        match *self {
            Op::Mul | Op::Div | Op::Add | Op::Sub => {
                check_arg!(0, &Type::Distr); check_arg!(1, &Type::Distr);
                Ok(Type::Distr)
            },
            Op::StatView | Op::HistView | Op::TableView => {
                check_arg!(0, &Type::Distr);
                Ok(Type::Nil)
            },
            Op::MakeDice => {
                check_arg!(0, &Type::Num); check_arg!(1, &Type::Num);
                Ok(Type::Distr)
            }
            Op::MakeDiceSingle => {
                check_arg!(0, &Type::Num);
                Ok(Type::Distr)
            }
        }
    }
    pub fn eval(&self, args: Vec<Cow<Symbol>>) -> Result<Symbol, Error> {
        Ok(match *self {
            Op::Mul => args[0].try_to_distr()?.combine_op(args[1].try_to_distr()?.as_ref(), |x, y| x * y).into(),
            Op::Div => args[0].try_to_distr()?.combine_fallible_op(args[1].try_to_distr()?.as_ref(),
                                                                   |x, y| x.checked_div(y).ok_or(fail!("zero division error")))?.into(),
            Op::Add => args[0].try_to_distr()?.combine_op(args[1].try_to_distr()?.as_ref(), |x, y| x + y).into(),
            Op::Sub => args[0].try_to_distr()?.combine_op(args[1].try_to_distr()?.as_ref(), |x, y| x - y).into(),
            Op::StatView  => println!("{}", args[0].try_to_distr()?.stat_view()).into(),
            Op::HistView  => println!("{}", args[0].try_to_distr()?.hist_view()).into(),
            Op::TableView => println!("{}", args[0].try_to_distr()?.table_view()).into(),
            Op::MakeDice => crate::distr::Distr::stacked_unifs(args[0].try_to_num()?.into_owned(), args[1].try_to_num()?.into_owned()).into(),
            Op::MakeDiceSingle => crate::distr::Distr::unif(args[0].try_to_num()?.into_owned()).into(),
        })
    }
}