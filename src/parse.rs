use crate::ptokens::{PToken, PTokenKind, MaybeHasSpan};
use crate::symbols::Symbol;
use crate::operations::Op;
use crate::env::Env;
use crate::error::Error;
use crate::error::ConcatErr;

use regex::Regex;

pub fn parse_line(src: &str, env: &Env) -> Result<Symbol, Error> {
    let mut iter = tokenize(src);
    let ptokens = build_pseudo_tokens(&mut iter, env, None, None)?;
    if let Some(result) = parse_assignment(&ptokens, env) {
        return result;
    }
    parse_expr(&ptokens, env)
}
/// Split a line into tokens as defined by the regex
fn tokenize(src: &str) -> impl Iterator<Item=regex::Match> + '_ {
    lazy_static! {
        static ref TOKEN_REGEX: Regex = Regex::new(r"([a-zA-Z\-_]+|[0-9]+|[\(\)\[\],]|[^\sA-Za-z0-9_\-]+)").expect("tokenization regex did not compile");
    }
    TOKEN_REGEX.find_iter(src)
}
/// Build up the vector of pseudo tokens from the token iterator, parsing everything inside parenthesis
fn build_pseudo_tokens<'a>(iter: &mut impl Iterator<Item=regex::Match<'a>>, env: &Env, start_paren: Option<usize>, start_brack: Option<usize>) -> Result<Vec<PToken>, Error> {
    let mut ptokens = vec![];
    loop {
        let mat= match iter.next() {
            None => {
                return if let Some(i) = start_paren {
                    Err(fail_at!((i,i+1), "unclosed parenthesis"))
                } else if let Some(i) = start_brack {
                    Err(fail_at!((i,i+1), "unclosed square brackets"))
                } else {
                    Ok(ptokens)
                }
            },
            Some(mat) => mat,
        };
        let value = match mat.as_str() {
            "(" => {
                // recursive call needs to remember where the start paren started.
                // forget that you're inside a square bracket for this scenario:
                // [   (   ]    )
                let inner_ptokens = build_pseudo_tokens(iter, env, Some(mat.start()), None)?;
                let symbol = if inner_ptokens.len() != 0 {
                    parse_expr(&inner_ptokens, env)?
                } else {
                    Symbol::Nil
                };
                // todo can we get the span from the first paren to the last paren?
                let span = inner_ptokens.as_slice().get_opt_span().unwrap_or((mat.start(), mat.end()));
                PToken::from_symbol(symbol, span)
            }
            ")" => {
                return if start_paren.is_some() {
                    Ok(ptokens)
                } else {
                    Err(fail_at!((mat.start(), mat.end()), "unmatched close parenthesis"))
                };
            }
            "[" => {
                // recursive call needs to remember where the brackets started.
                // forget that you're inside a paren for this scenario:
                // [   (   ]    )
                let inner_ptokens = build_pseudo_tokens(iter, env, None, Some(mat.start()))?;
                let symbol = parse_seq(&inner_ptokens, env)?;
                // todo can we get the span from the first paren to the last paren?
                let span = inner_ptokens.as_slice().get_opt_span().unwrap_or((mat.start(), mat.end()));
                PToken::from_symbol(symbol, span)
            }
            "]" => {
                return if start_brack.is_some() {
                    Ok(ptokens)
                } else {
                    Err(fail_at!((mat.start(), mat.end()), "unmatched close_parenthesis"))
                }
            }
            _ => PToken::from(mat.as_str().to_string(), (mat.start(), mat.end())),
        };
        ptokens.push(value);
    }
}

/// Parse an assignment statement
/// Return Some(result) if it's definitely an assignment statement
/// Return None if not
fn parse_assignment(ptokens: &[PToken], env: &Env) -> Option<Result<Symbol, Error>> {
    let segments = ptokens
        .split(|ptoken| ptoken.try_to_reserved().map_or(false, |k| k == "="))
        .collect::<Vec<&[PToken]>>();
    match segments.len() {
        2 => {
            let pat = match parse_pat(segments[0]).concat_err(fail!("invalid left hand side of assignment statement")) {
                Ok(pat) => pat,
                Err(e) => return Some(Err(e)),
            };
            let expr = match parse_expr(segments[1], env).concat_err(fail!("invalid right hand side of assignment statement")) {
                Ok(expr) => expr,
                Err(e) => return Some(Err(e)),
            };
            Some(Ok(Symbol::Assigner {
                name: pat,
                def_type: None,
                expr: expr.into_boxed(),
            }))
        },
        0 | 1 => None, // we pass: something else will take care of this
        _ => Some(Err(fail_at!(ptokens.get_opt_span().unwrap(), "can't nest assignments: `=` is not an operator"))),
    }
}

fn parse_expr(ptokens: &[PToken], env: &Env) -> Result<Symbol, Error> {
    // can't parse nothing
    if ptokens.is_empty() {
        return Err(fail!("unexpected EOF while parsing expr"));
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
        if let PTokenKind::Reserved(ref curr_kwrd) = &ptokens[i].kind {
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
    let (kwrd, idx, _) = match lowest {
        Some(info) => info,
        None => return Err(fail_at!(ptokens.get_opt_span().expect("ptokens of len 1 and len 0 should be checked already"), "no operator found here"))
    };
    let parse_left = || parse_expr(&ptokens[..idx], env).concat_err(fail_at!(ptokens[idx].span, "Could not parse left hand operand of operator `{}`", kwrd));
    let parse_right = || parse_expr(&ptokens[idx+1..], env).concat_err(fail_at!(ptokens[idx].span, "Could not parse right hand operand of operator `{}`", kwrd));
    Ok(
        match kwrd {
            "d" if idx == 0 => Symbol::ApplyBuiltin(vec![parse_right()?], Op::MakeDiceSingle),
            "d" => Symbol::ApplyBuiltin(vec![parse_left()?, parse_right()?], Op::MakeDice),
            "*" => Symbol::ApplyBuiltin(vec![parse_left()?, parse_right()?], Op::Mul),
            "/" => Symbol::ApplyBuiltin(vec![parse_left()?, parse_right()?], Op::Div),
            "+" => Symbol::ApplyBuiltin(vec![parse_left()?, parse_right()?], Op::Add),
            "-" => Symbol::ApplyBuiltin(vec![parse_left()?, parse_right()?], Op::Sub),
            ">>" => Symbol::Apply{exprs: vec![parse_left()?], func: parse_right()?.into_boxed()}, // todo multiple input: is this part of the same deal as partial application?
            op => return Err(fail_at!(ptokens[idx].span, "no operator found with name: {}", op)),
        }
    )
}

/// parse a sequence literal: [1, 2, 3] or [1; 4] and the like
fn parse_seq(ptokens: &[PToken], env: &Env) -> Result<Symbol, Error> {
    if ptokens.len() == 0 {
        return Ok(Symbol::Seq(vec![]));
    }
    // todo: parse other seq literals: [d6; 4] and [0..6]
    // parse semi colon syntax: [d6; 4]
    let segments = ptokens
        .split(|ptoken| ptoken.try_to_reserved().map_or(false, |k| k == ";"))
        .collect::<Vec<&[PToken]>>();
    match segments.len() {
        2 => return parse_expr(segments[0], env).concat_err(fail!("could not parse sequence")),
        0 | 1 => { } // pass: comma separated list will take care of this
        _ => return Err(fail_at!(ptokens.get_opt_span().unwrap(), "unexpected sequence syntax: too many semicolons")),
    }

    // parse comma separated seq: [3, 4, 5]
    let vec = ptokens
        .split(|ptoken| ptoken.try_to_reserved().map_or(false, |k| k == ","))
        .map(|segment| parse_expr(segment, env).concat_err(fail!("could not parse as comma separated sequence")))
        .collect::<Result<Vec<Symbol>, Error>>()?;
    Ok(Symbol::Seq(vec))
}

fn parse_pat(ptokens: &[PToken]) -> Result<String, Error> {
    match ptokens.len() {
        1 => {
            if let PTokenKind::Expr(Symbol::Text(ref string)) = &ptokens[0].kind {
                Ok(string.to_string())
            } else {
                Err(fail_at!(ptokens[0].span, "not valid pattern"))
            }
        }
        0 => Err(fail!("unexpected EOF while parsing pattern")),
        _ => Err(fail_at!(ptokens.get_opt_span().unwrap(), "not valid pattern (too many tokens)"))
    }
}