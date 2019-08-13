#[macro_use]
extern crate lazy_static;

#[macro_use]
mod error;
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

fn prompt_user(prompt: &str) -> io::Result<String> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    write!(&mut stdout, "{}", prompt)?;
    stdout.flush()?;
    let mut buf = String::new();
    stdin.read_line(&mut buf)?;
    Ok(buf)
}

/// read, evaluate, and then print the users line
pub fn read_eval_print(line: &str, env: &mut Env) -> Result<Symbol, Error> {
    let tokens = parse::tokenize(&line);
   /* if tokens.len() == 0 {
        if prompt_user("Quit session? [y/n]: ").unwrap().to_lowercase().trim_end() == "y" {
            return Ok(())
        } else {
            println!();
            continue
        }
    } */
    println!("tokens: {:?}", tokens);
    println!("environment: {:?}", env);
    let ast: Symbol = parse::parse_line(&tokens, &env)?;
    let type_ = ast.type_check(&env)?;
    ast.walk(0);
    println!("{}", ast.repr());
    println!("=>{:?}", type_);
    println!();
    let res = ast.eval(env).into_owned();
    println!("{:?}", res);
    Ok(res)
}
fn main() {
    println!("opening dnd calculator session");
    use type_info::Type;
    let mut env = Env::new();
    env
        .bind_var("ZERO".to_string(), Symbol::from(0), Type::Num);
//        .bind_var(vec!["func-name".to_string(), "arg1".to_string()], Symbol::from("arg1".to_string()));
    loop {
        let line = prompt_user("/>  ").unwrap();
        if let Err(err_msg) = read_eval_print(&line, &mut env) {
            err_msg.pretty_print();
        };
    }
}
