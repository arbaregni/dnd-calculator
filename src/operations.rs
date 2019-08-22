use crate::type_info::{Type, FnType};
use crate::symbols::Symbol;
use crate::error::Error;
use crate::distr::Distr;

use std::borrow::Cow;
use crate::env::Env;

impl Env {
    pub fn import_arithmetic(&mut self) -> &mut Self {
        self
            // MULTIPLICATION
            .bind_fn_var("mul".to_string(), Box::new(|args| {
                args[0].try_to_distr().and_then(|left|
                    args[1].try_to_distr().and_then(|right|
                        Ok(left.as_ref().combine_op(right.as_ref(), |x, y| x * y).into())
                    )
                ).expect("Symbol::Fn doesn't support error handling yet")
            }), fn_type!(Type::Distr, Type::Distr, -> Type::Distr))
        ;
        self
    }
}

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
    pub fn eval(&self, args: Vec<Cow<Symbol>>) -> Result<Symbol, Error> {
        Ok(match *self {
            _ => unimplemented!(),
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