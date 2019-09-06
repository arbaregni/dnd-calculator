use crate::type_info::{Type};
use crate::error::Error;
use crate::closures::FnType;

use crate::env::Env;

impl Env {
    pub fn import_arithmetic(&mut self) -> &mut Self {
        self
            // MULTIPLICATION
            .bind_fn_var("mul".to_string(), |args, _| {
                args[0].try_to_distr().and_then(|left|
                    args[1].try_to_distr().and_then(|right|
                        Ok(left.as_ref().combine_op(right.as_ref(), |x, y| x * y).into())
                    )
                )
            }, fn_type!(Type::Distr, Type::Distr, -> Type::Distr))
            // DIVISION
            .bind_fn_var("div".to_string(), |args, _| {
                args[0].try_to_distr().and_then(|left|
                    args[1].try_to_distr().and_then(|right|
                        Ok(left.as_ref().combine_op(right.as_ref(), |x, y| x / y).into())
                    )
                )
            }, fn_type!(Type::Distr, Type::Distr, -> Type::Distr))
            // SUBTRACTION
            .bind_fn_var("sub".to_string(), |args, _| {
                args[0].try_to_distr().and_then(|left|
                    args[1].try_to_distr().and_then(|right|
                        Ok(left.as_ref().combine_op(right.as_ref(), |x, y| x - y).into())
                    )
                )
            }, fn_type!(Type::Distr, Type::Distr, -> Type::Distr))
            // ADDITION
            .bind_fn_var("add".to_string(), |args, _| {
                args[0].try_to_distr().and_then(|left|
                    args[1].try_to_distr().and_then(|right|
                        Ok(left.as_ref().combine_op(right.as_ref(), |x, y| x + y).into())
                    )
                )
            }, fn_type!(Type::Distr, Type::Distr, -> Type::Distr))
        ;
        self
    }
    pub fn import_dice(&mut self) -> &mut Self {
        self
            // MAKE DICE
            .bind_fn_var("make-dice".to_string(), |args, _| {
                args[0].try_to_num().and_then(|k|
                    args[1].try_to_num().and_then(|n|
                        Ok(crate::distr::Distr::stacked_unifs(k.into_owned(), n.into_owned()).into())
                    )
                )
            }, fn_type!(Type::Num, Type::Num, -> Type::Distr))
            // TABLE VIEW
            .bind_fn_var("table".to_string(), |args,_| {
                args[0].try_to_distr().and_then(|distr|
                    Ok(println!("{}", distr.table_view()).into())
                )
            }, fn_type!(Type::Distr, -> Type::Nil))
            // HIST VIEW
            .bind_fn_var("hist".to_string(), |args, _| {
                args[0].try_to_distr().and_then(|distr|
                    Ok(println!("{}", distr.hist_view()).into())
                )
            }, fn_type!(Type::Distr, -> Type::Nil))
        ;
        self
    }
}