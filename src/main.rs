#[macro_use]
extern crate lazy_static;

#[macro_use]
mod error;
mod ptokens;
mod type_info;
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
use crate::error::Error;
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
    let ast: Symbol = parse::parse_line(line, env)?;
    let type_ = ast.type_check(&env)?;
    ast.walk(env, 0);
    println!("{}", ast.repr());
    println!("=>{:?}", type_);
    println!();
    let res = ast.eval(env)?.into_owned();
    println!("{:?}", res);
    Ok(res)
}
fn main() {
    println!("opening dnd calculator session");
    use type_info::Type;
    let mut env = Env::new();
    env
        .bind_fn_var("I".to_string(), Symbol::Fn(Box::new(|vec| vec[0].clone()),
                                                 FnType{ in_types: vec![Type::Distr], out_type: Box::new(Type::Distr) }))
        .bind_fn_var("K".to_string(), Symbol::Fn(Box::new(|vec| vec[1].clone()),
                                                 FnType{ in_types: vec![Type::Distr, Type::Distr], out_type: Box::new(Type::Distr) }));
    loop {
        let line = prompt_user("/>  ").unwrap();
        if let Err(err) = read_eval_print(&line, &mut env) {
            if let Some(span) = err.opt_span {
                println!("{}", Error::underline(&line, span));
            }
            println!("{}", err);
        };
    }
}
