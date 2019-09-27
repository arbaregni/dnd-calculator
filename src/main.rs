extern crate pad;
#[macro_use] extern crate lazy_static;
extern crate pest;
#[macro_use] extern crate pest_derive;

#[macro_use] mod error;
#[macro_use] mod closures;
mod util;
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

struct Flags {
    pub filename: Option<String>,
    pub debug: bool,
}
impl Flags {
    pub fn get() -> Flags {
        let mut flags = Flags {
            filename: None,
            debug: false,
        };
        let mut args = std::env::args();
        if let Some(name) = args.next() {
            flags.filename = Some(name);
        }
        for arg in args {
            match arg.as_str() {
                "--debug" | "-d" => flags.debug = true,
                _ => {}
            }
        }
        flags
    }
}

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
pub fn parse_analyze_evaluate(line: &str, env: &mut Env, debug: bool) -> Result<Symbol, Error> {
    if debug { println!("----------------------------"); }
    if debug { env.print(); }
    let ast: Symbol = parse::parse_line(line, env)?;
    if debug { ast.walk(env, 0); }
    let type_ = ast.type_check(&env).concat_err(fail!("type checker failed"))?;
    if debug {
        println!("{}", ast.repr());
        println!("=>{}", type_);
    }
    if debug { println!("----------------------------"); }
    Ok(ast.eval(env).concat_err(fail!("evaluator failed"))?.into_owned())
}
fn main() {
    let flags = Flags::get();
    println!("opening dnd calculator session");
    use type_info::Type;
    let mut env = Env::new();
    env
        .import_arithmetic()
        .import_dice()
        .import_comparisons()
        .bind_fn_var("debug".to_string(), |vec, _| {
            Ok(println!("{:#?}", vec[0]).into())
        }, fn_type!(Type::Any, -> Type::Any))
        ;
    loop {
        let line = prompt_user("/>  ").unwrap();
        if line.trim() == "exit" { break; }
        if line.trim().len() == 0 { continue; }
        let res = parse_analyze_evaluate(&line, &mut env, flags.debug);
        match res {
            Ok(symbol) => {
                if flags.debug { println!("   {:?}", symbol); }
                println!("{}", symbol.repr());
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
