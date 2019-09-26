#[macro_use] extern crate lazy_static;
extern crate pest;
#[macro_use] extern crate pest_derive;

#[macro_use] mod error;
#[macro_use] mod closures;
mod type_info;
mod distr;
mod env;
mod std_imports;
mod parse;
mod symbols;


#[cfg(test)]
mod tests;

use std::io;
use std::io::Write;

use symbols::Symbol;
use crate::error::{Error, ConcatErr};
use crate::env::Env;
use crate::closures::FnType;

fn prompt_user(prompt: &str) -> io::Result<String> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    write!(&mut stdout, "{}", prompt)?;
    stdout.flush()?;
    let mut buf = String::new();
    stdin.read_line(&mut buf)?;
    Ok(buf.trim().to_string())
}

/// Take a line of input and convert it into a symbol, performing type analysis along the way
pub fn parse_analyze_evaluate(line: &str, env: &mut Env) -> Result<Symbol, Error> {
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
        .import_arithmetic()
        .import_dice()
        .bind_fn_var("debug".to_string(), |vec, _| {
            Ok(println!("{:#?}", vec[0]).into())
        }, fn_type!(Type::Any, -> Type::Any)
        )
        ;
    loop {
        let line = prompt_user("/>  ").unwrap();
        if line.trim() == "exit" { break; }
        println!("-------------------------");
        let res = parse_analyze_evaluate(&line, &mut env);
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
