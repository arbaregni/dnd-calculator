use crate::symbols::Symbol;
use crate::distr::{KeyType};
use crate::operations::Op;
use crate::env::Env;
use crate::error::Error;
use crate::type_info::Type;

use regex::Regex;

type Parse<'a, T> = Result<(T, &'a [String]), Error>;

/// Split a line into tokens as defined by the regex
pub fn tokenize(src: &str) -> Vec<String> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"[a-zA-Z\-_]+|[0-9]+|[\(\)\[\],]|[^\sA-Za-z0-9_\-]+").expect("tokenization regex did not compile");
    }
    RE.captures_iter(src)
        .map(|capt| capt[0].to_string())
        .collect()
}

/// match the first literal of the token stream to a literal
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
/// return the section of tokens before the first instance of the literal
fn scan_for_literal<'a>(tokens: &'a [String], literal: &str) -> Parse<'a, &'a [String]> {
    let mid = tokens
        .iter()
        .position(|token| *token == literal)
        .ok_or(fail!("could not find literal `{}`", literal))?;
    let (left, rest) = tokens.split_at(mid);
    Ok((left, &rest[1..]))
}
/// Parse out a number or an identifier
fn parse_atom(tokens: &[String]) -> Parse<Symbol> {
    let (token, _rest) = tokens.split_first()
        .ok_or(fail!("encountered EOF while parsing"))?;
    let symbol = match token.parse::<KeyType>() {
        Ok(num) => Symbol::Num(num),
        Err(_)  => Symbol::Text(token.to_string()),
    };
    Ok((symbol, &tokens[1..]))
}

/// extract the enclosed section of tokens in between two balanced symbols: i.e. ( )
fn extract_from_parens<'a>(tokens: &'a [String], open: &'static str, close: &'static str) -> Parse<'a, &'a [String]> {
    let (_, rest) = match_literal(tokens, open)?;
    let mut lvl = 1;
    for i in 0..rest.len() {
        if rest[i].as_str() == open {
            lvl += 1;
        }
        if rest[i].as_str() == close {
            lvl -= 1;
            if lvl == 0 {
                return Ok((&rest[0..i], &rest[i+1..]));
            }
        }
    }
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

fn parse_seq<'a>(tokens: &'a [String], env: &Env) -> Parse<'a, Symbol> {
    let (inside, rest) = extract_from_parens(tokens, "[", "]")?;
    // todo [d6; 4] syntax, [1..5] syntax
    let mut symbol_vec = vec![];
    if !inside.is_empty() {
        for sections in inside.split(|token| token == ",") {
            let (expr, leftover ) = parse_expr(sections, env)?;
            if leftover.len() != 0 { return Err(fail!("unexpected tokens after expr inside sequence literal: {:?}", leftover))}
            symbol_vec.push(expr);
        }
    }
    Ok((Symbol::Seq(symbol_vec), rest))
}

fn parse_atom_or_parens<'a>(tokens: &'a [String], env: &Env) -> Parse<'a, Symbol> {
    match tokens.get(0).map(String::as_str) {
        None => Err(fail!("unexpected EOF while parsing")),
        Some("(") => {
            let (inside, rest) = extract_from_parens(tokens, "(", ")")?;
            let (expr, leftover) = parse_expr(inside, env)?;
            if leftover.len() != 0 { return Err(fail!("unexpected tokens after expr inside parens: {:?}", leftover))}
            return Ok((expr, rest))
        },
        Some(_) => parse_atom(tokens),
    }
}

fn parse_after<'a>(front_expr: Symbol, tokens: &'a [String], env: &Env) -> Parse<'a, Symbol> {
    let (expr, rest) = match tokens.get(0).map(String::as_str) {
        // there's nothing after a valid expr : that's ok, just return the valid expr
        None => return Ok((front_expr, tokens)),
        // parse everything after the infix operation and wrap it up with the front expr in a compound symbol
        Some("*") => parse_atom_or_parens(&tokens[1..], env).map(|(back_expr, rest)| (Symbol::ApplyBuiltin(vec![front_expr, back_expr], Op::Mul), rest)),
        Some("/") => parse_atom_or_parens(&tokens[1..], env).map(|(back_expr, rest)| (Symbol::ApplyBuiltin(vec![front_expr, back_expr], Op::Div), rest)),
        Some("+") => parse_expr(&tokens[1..], env).map(|(back_expr, rest)| (Symbol::ApplyBuiltin(vec![front_expr, back_expr], Op::Add), rest)),
        Some("-") => parse_expr(&tokens[1..], env).map(|(back_expr, rest)| (Symbol::ApplyBuiltin(vec![front_expr, back_expr], Op::Sub), rest)),
        Some(">>") => parse_atom_or_parens(&tokens[1..], env).map(|(back_expr, rest)| (Symbol::Apply {exprs: vec![front_expr], func: back_expr.into_boxed()}, rest)),
        Some("d") => parse_atom_or_parens(&tokens[1..], env).map(|(back_expr, rest)| (Symbol::ApplyBuiltin(vec![front_expr, back_expr], Op::MakeDice), rest)),
        Some(op) => Err(fail!("unrecognized operation `{}`", op)),
    }?;
    parse_after(expr, rest, env)
}

fn parse_expr<'a>(tokens: &'a [String], env: &Env) -> Parse<'a, Symbol> {
    println!("---parsing : {:?}", tokens);
    let word = match tokens.get(0) {
        Some(s) => s.clone(),
        None => return Err(fail!("unexpected EOF while parsing")),
    };
    // attempt to match the first expr
    let (front_expr, rest) = match word.as_str() {
        "d" => {
            let (expr, rest) = parse_atom_or_parens(&tokens[1..], env)?;
            (Symbol::ApplyBuiltin(vec![expr], Op::MakeDiceSingle), rest)
        },
        "[" => parse_seq(tokens, env)?,
        _   => parse_atom_or_parens(tokens, env)?,
    };
    // check the next token for infix operations
    parse_after(front_expr, rest, env)
}

pub fn parse_line(tokens: &[String], env: &Env) -> Result<Symbol, Error> {
    let (symbol, remaining) =
        // if we contain an equals token, then we're an assignment and not an expression
        if tokens.iter().find(|token| *token == "=").is_some() {
            if tokens[0] == "fn" {
                parse_fn_dec(tokens, env)
            } else {
                parse_dec(tokens, env)
            }
        } else {
            // otherwise, it's just a normal expr
            parse_expr(tokens, env)
        }?;
    if remaining.len() != 0 {
        return Err(fail!("unexpected tokens: {:?}, after symbol: {:?}", remaining, symbol));
    }
    Ok(symbol)
}