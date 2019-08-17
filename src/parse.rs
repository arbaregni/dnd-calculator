use crate::symbols::Symbol;
use crate::distr::{KeyType};
use crate::operations::Op;
use crate::env::Env;
use crate::error::Error;

use regex::Regex;

#[derive(Debug)]
enum PToken {
    Reserved(String),
    Expr(Symbol),
}
impl PToken {
    fn from(raw: String) -> PToken {
        match raw.parse::<KeyType>() {
            Ok(num) => PToken::Expr(num.into()),
            Err(_)  => match raw.as_str() {
                "*" | "/" | "+" | "-" | "d" | ">>" | "(" | ")" | "[" | "]" | "," | ";" => PToken::Reserved(raw),
                _ => PToken::Expr(raw.into())
            }
        }
    }
    fn try_to_expr(&self) -> Option<Symbol> {
        match *self {
            PToken::Reserved(_) => None,
            PToken::Expr(ref symbol) => Some(symbol.clone()),
        }
    }
}
/// Split a line into tokens as defined by the regex
fn tokenize(src: &str) -> impl Iterator<Item=String> + '_ {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"[a-zA-Z\-_]+|[0-9]+|[\(\)\[\],]|[^\sA-Za-z0-9_\-]+").expect("tokenization regex did not compile");
    }
    RE.captures_iter(src).map(|capt| capt[0].to_string())
}
/// Build up the vector of pseudo tokens from the token iterator, parsing everything inside parenthesis
fn build_pseudo_tokens<'a>(iter: &mut impl Iterator<Item=String>, env: &Env, in_parens: bool) -> Result<Vec<PToken>, Error> {
    let mut ptokens = vec![];
    loop {
        let raw = match iter.next() {
            None => {
                return if !in_parens {
                    Ok(ptokens)
                } else {
                    Err(fail!("unclosed parenthesis"))
                }
            },
            Some(raw) => raw,
        };
        match raw.as_str() {
            "(" => {
                let inner_ptokens = build_pseudo_tokens(iter, env,true)?;
                let expr = parse_expr(&inner_ptokens, env)?;
                ptokens.push(PToken::Expr(expr));
                continue;
            }
            ")" => {
                return if in_parens {
                    Ok(ptokens)
                } else {
                    Err(fail!("unmatched close parenthesis"))
                };
            }
            _ => {},
        };
        ptokens.push(PToken::from(raw));
    }
}

fn parse_expr(ptokens: &[PToken], env: &Env) -> Result<Symbol, Error> {
    // can't parse nothing
    if ptokens.is_empty() {
        return Err(fail!("unexpected EOF while parsing"));
    }
    // if we get one ptoken left, and it's an expr, we're done: no recursion
    if ptokens.len() == 1 {
        if let Some(symbol) = ptokens[0].try_to_expr() {
            return Ok(symbol);
        }
    }
    // scan the pseudo tokens for the lowest precedence operator, its index, and its precedence value
    // store the lowest precedence (keyworded text, index, precedence)
    let mut lowest: Option<(&str, usize, u32)> = None; // todo creating a sorted list of operators be more efficient?
    for i in 0..ptokens.len() {
        if let PToken::Reserved(ref curr_kwrd) = &ptokens[i] {
            let curr_prec = match curr_kwrd.as_str() {
                "d" => 100,
                "*" | "/" => 55,
                "+" | "-" => 54,
                ">>" => 0,
                _ => panic!("unexpected keyword: {} (could not assign precedence)", curr_kwrd)
            };
            if let Some((_, _, prec)) = lowest {
                // the current precedence must be strictly greater in order to default to the left most operator
                if curr_prec <= prec {
                    lowest = Some((curr_kwrd, i, curr_prec));
                }
            } else {
                lowest = Some((curr_kwrd, i, curr_prec));
            }
        }
    }
    // todo handle this error
    let (kwrd, idx, _) = lowest.expect("can't handle not finding an operator");
    let parse_left = || parse_expr(&ptokens[..idx], env).map_err(|err| enrich!(err, "Could not parse left hand operand of operator `{}`", kwrd));
    let parse_right = || parse_expr(&ptokens[idx+1..], env).map_err(|err| enrich!(err, "Could not parse right hand operand of operator `{}`", kwrd));
    Ok(
        match kwrd {
            "d" if idx == 0 => Symbol::ApplyBuiltin(vec![parse_right()?], Op::MakeDiceSingle),
            "d" => Symbol::ApplyBuiltin(vec![parse_left()?, parse_right()?], Op::MakeDice),
            "*" => Symbol::ApplyBuiltin(vec![parse_left()?, parse_right()?], Op::Mul),
            "/" => Symbol::ApplyBuiltin(vec![parse_left()?, parse_right()?], Op::Div),
            "+" => Symbol::ApplyBuiltin(vec![parse_left()?, parse_right()?], Op::Add),
            "-" => Symbol::ApplyBuiltin(vec![parse_left()?, parse_right()?], Op::Sub),
            ">>" => Symbol::Apply{exprs: vec![parse_left()?], func: parse_right()?.into_boxed()}, // todo multiple input: is this part of the same deal as partial application?
            op => return Err(fail!("no operator found with name: {}", op)),
        }
    )
}

pub fn parse_line(src: &str, env: &Env) -> Result<Symbol, Error> {
    let mut iter = tokenize(src);
    let ptokens = build_pseudo_tokens(&mut iter, env, false)?;
    parse_expr(&ptokens, env)
}