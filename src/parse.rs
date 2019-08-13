use crate::symbols::Symbol;
use crate::distr::{KeyType};
use crate::operations::Op;
use crate::env::Env;
use crate::error::Error;
use crate::type_info::Type;

use regex::Regex;

type Parse<'a, T> = Result<(T, &'a [String]), Error>;
type SymbolMaker = Box<dyn Fn(Vec<Symbol>) -> Symbol>;

pub fn tokenize(src: &str) -> Vec<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"[a-zA-Z\-_]+|[0-9]+|[\(\)\[\]]|[^\sA-Za-z0-9_\-]+").expect("tokenization regex did not compile");
    }
    RE.captures_iter(src)
        .map(|capt| capt[0].to_string())
        .collect()
}

fn parse_atom(tokens: &[String]) -> Parse<Symbol> {
    let (token, _rest) = tokens.split_first()
        .ok_or(fail!("encountered EOF while parsing"))?;
    let symbol = match token.parse::<KeyType>() {
        Ok(num) => Symbol::Num(num),
        Err(_)  => Symbol::Text(token.to_string()),
    };
    Ok((symbol, &tokens[1..]))
}
fn match_literal<'a>(tokens: &'a [String], literal: &str) -> Parse<'a, &'a String> {
    let (token, rest) = tokens
        .split_first()
        .ok_or(fail!("encountered EOF while parsing (expected literal `{}`)", literal))?;
    if *token == literal {
        Ok((token, rest))
    } else {
        Err(fail!("expected literal `{}` -- found token `{}`", literal, token))
    }
}
fn scan_for_literal<'a>(tokens: &'a [String], literal: &str) -> Parse<'a, &'a [String]> {
    let mid = tokens
        .iter()
        .position(|token| *token == literal)
        .ok_or(fail!("could not find literal `{}`", literal))?;
    let (left, rest) = tokens.split_at(mid);
    Ok((left, &rest[1..]))
}


fn parse_prefix<'a>(tokens: &'a [String], env: &Env, literal: &str, maker: SymbolMaker) -> Parse<'a, Symbol> {
    let (_, rest) = match_literal(tokens, literal)?;
    let (symbol, rest) = parse_expr(rest, env)?;
    return Ok((maker(vec![symbol]), rest));
}



fn parse_parens<'a>(tokens: &'a [String], env: &Env, open: &'static str, close: &'static str) -> Parse<'a, Symbol> {
    let (_, rest) = match_literal(tokens, open)?;
    let mut lvl = 1;
    for i in 0..rest.len() {
        if rest[i].as_str() == open {
            lvl += 1;
        }
        if rest[i].as_str() == close {
            lvl -= 1;
            if lvl == 0 {
                let (expr, leftover) = parse_expr(&rest[0..i], env)?;
                if leftover.len() != 0 { return Err(fail!("unexpected tokens after expr inside parens: {:?}", rest))}
                return Ok((expr, &rest[i+1..]))
            }
        }
    }
    dbg!(tokens);
    dbg!(lvl);
    Err(fail!("unmatched parenthesis"))
}

fn parse_dec<'a>(tokens: &'a [String], env: &Env) -> Parse<'a, Symbol> {
    let (name, rest) = tokens
        .split_first()
        .ok_or(fail!("encountered EOF while parsing declaration (expected identifier)"))?;
    let (_, rest) = match_literal(rest, "=")?;
    let (expr, rest) = parse_expr(rest, env)?;
    Ok((Symbol::Assigner{name: name.to_string(), def_type: None, expr: Box::new(expr)}, rest))
}

fn parse_fn_dec<'a>(tokens: &'a [String], env: &Env) -> Parse<'a, Symbol> {
    let (_, rest) = match_literal(tokens, "fn")?;
    let (name, rest) = rest
        .split_first()
        .ok_or(fail!("encountered EOF while parsing function declaration (expected a name)"))?;
    println!("name: {}, rest: {:?}", name, rest);
    let (signature_tokens, rest) = scan_for_literal(rest, "=")?; //TODO we can check if it's a block function
    println!("{:?}", signature_tokens);
    let mut child_env = Env::new();
    let mut iter = signature_tokens.iter().peekable();
    while iter.peek().is_some() {
        let arg_name = iter.next().ok_or(fail!("expected argument name in function signature"))?;
        let sep = iter.next().ok_or(fail!("expected type annotation in function signature"))?;
        if sep != ":" {
            return Err(fail!("unexpected separator after argument name in function signature `{}` (expected `:`)", sep));
        }
        let type_token = iter.next().ok_or(fail!("expected argument type in function signature following argument `{}`", arg_name))?;
        let type_ = crate::type_info::Type::try_from(type_token).ok_or(fail!("unknown type in function declaration argument: {}", type_token))?;
        child_env.bind_var(arg_name.clone(), Symbol::Nil, type_);
    }
    let (expr, rest) = parse_expr(rest, env)?;
    Ok((Symbol::Assigner{name: name.to_string(), def_type: Some(Type::Nil), expr: Box::new(expr)}, rest))
}

fn parse_atom_or_parens<'a>(tokens: &'a [String], env: &Env) -> Parse<'a, Symbol> {
    match tokens.get(0).map(String::as_str) {
        None => Err(fail!("unexpected EOF while parsing")),
        Some("(") => parse_parens(tokens, env, "(", ")"),
        Some(_) => parse_atom(tokens),
    }
}

fn parse_infix_after<'a>(result: Parse<'a, Symbol>, env: &Env) -> Parse<'a, Symbol> {
    let (expr0, rest) = result?;
    let (op, (expr1, rest)) = match rest.get(0).map(String::as_str) {
        // there's nothing after a valid expr : that's ok, just return the valid expr
        None => return Ok((expr0, rest)),
        Some("*") => (Op::Mul, parse_expr(&rest[1..], env)?),
        Some("/") => (Op::Div, parse_expr(&rest[1..], env)?),
        Some("+") => (Op::Add, parse_expr(&rest[1..], env)?),
        Some("-") => (Op::Sub, parse_expr(&rest[1..], env)?),
        Some("d") => (Op::MakeDice, parse_atom_or_parens(&rest[1..], env)?),
        Some(op) => Err(fail!("unrecogized operation `{}`", op))
    };
    parse_infix_after(Ok((Symbol::ApplyBuiltin(vec![expr0, expr1], op), rest)), env)
}
fn parse_expr<'a>(tokens: &'a [String], env: &Env) -> Parse<'a, Symbol> {
    println!("---parsing : {:?}", tokens);
    let word = match tokens.get(0) {
        Some(s) => s.clone(),
        None => return Err(fail!("unexpected EOF while parsing")),
    };
    match word.as_str() {
        "d"    => {
            let (expr, rest) = parse_atom_or_parens(&tokens[1..], env)?;
            Ok((Symbol::ApplyBuiltin(vec![expr], Op::MakeDiceSingle), rest))
        },
        _ => parse_infix_after(parse_atom_or_parens(tokens, env), env),
    }
}

pub fn parse_line(tokens: &[String], env: &Env) -> Result<Symbol, Error> {
    let (symbol, remaining) =
        // if we contain an equals token, then we're an assignment and not an expression
        if tokens.iter().position(|token| token == "=").is_some() {
            if tokens[0] == "fn" {
                parse_fn_dec(tokens, env)
            } else {
                parse_dec(tokens, env)
            }
        } else {
            parse_expr(tokens, env)
        }?;
    if remaining.len() != 0 {
        return Err(fail!("unexpected tokens: {:?}, after symbol: {:?}", remaining, symbol));
    }
    Ok(symbol)
}