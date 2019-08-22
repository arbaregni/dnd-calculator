#[macro_use] extern crate lazy_static;

#[macro_use] mod error;
#[macro_use] mod type_info;
mod ptokens;
mod operations;
mod env;
mod parse;
mod symbols;
mod distr;

#[cfg(test)]
mod tests;

use std::io;
use std::io::Write;

use symbols::Symbol;
use crate::error::{Error, ConcatErr};
use crate::env::Env;
use crate::type_info::FnType;

fn prompt_user(prompt: &str) -> io::Result<String> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    write!(&mut stdout, "{}", prompt)?;
    stdout.flush()?;
    let mut buf = String::new();
    stdin.read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

/// read, evaluate, and then print the users line
pub fn read_eval_print(line: &str, env: &mut Env) -> Result<Symbol, Error> {
    println!("environment: {:?}", env);
    let ast: Symbol = parse::parse_line(line, env).concat_err(fail!("parser failed"))?;
    ast.walk(env, 0);
    let type_ = ast.type_check(&env).concat_err(fail!("type checker failed"))?;
    println!("{}", ast.repr());
    println!("=>{}", type_);
    Ok(ast.eval(env).concat_err(fail!("evaluator failed"))?.into_owned())
}
fn main() {
    println!("opening dnd calculator session");
    use type_info::Type;
    let mut env = Env::new();
    env
        .bind_fn_var("I".to_string(), Box::new(|vec| vec[0].clone()),
                                                 fn_type!(Type::Distr, -> Type::Distr)
        )
        .bind_fn_var("K".to_string(), Box::new(|vec| vec[0].clone()),
                                                 fn_type!(Type::Distr, Type::Distr, -> Type::Distr)
        );
    loop {
        let line = prompt_user("/>  ").unwrap();
        println!("-------------------------");
        let res = read_eval_print(&line, &mut env);
        println!("-------------------------");
        match res {
            Ok(symbol) => {
                println!(" {:?}\n{}", symbol, symbol.repr());
            }
            Err(err) => {
                if let Some(span) = err.opt_span {
                    println!("{}", Error::underline(&line, span));
                }
                println!("{}", err);
            }
        }
    }
}
