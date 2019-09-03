use crate::ptokens::{PToken, PTokenKind, MaybeHasSpan, TokenCaptureStream};
use crate::symbols::Symbol;
use crate::env::Env;
use crate::error::Error;
use crate::error::ConcatErr;
use crate::type_info::{Type, FnType};

pub fn parse_line(src: &str, env: &Env) -> Result<Symbol, Error> {
    let mut iter = TokenCaptureStream::new(src);
    let ptokens = build_pseudo_tokens(&mut iter, env, None, None)?;
    if let Some(result) = parse_assignment(&ptokens, env) {
        return result;
    }
    parse_expr(&ptokens, env)
}
/// Build up the vector of pseudo tokens from the token iterator, parsing everything inside parenthesis
fn build_pseudo_tokens<'a>(iter: &mut impl Iterator<Item=Result<PToken, Error>>, env: &Env, start_paren: Option<usize>, start_brack: Option<usize>) -> Result<Vec<PToken>, Error> {
    let mut ptokens = vec![];
    loop {
        let ptoken = match iter.next() {
            Some(result) => result?,
            None => {
                return if let Some(i) = start_paren {
                    Err(fail_at!((i,i+1), "unclosed parenthesis"))
                } else if let Some(i) = start_brack {
                    Err(fail_at!((i,i+1), "unclosed square brackets"))
                } else {
                    Ok(ptokens)
                }
            }
        };
        println!("token: {:?}", ptoken);
        let value = if ptoken.is_reserved("(") {
            // recursive call needs to remember where the start paren started.
            // forget that you're inside a square bracket for this scenario:
            // [   (   ]    )
            let inner_ptokens = build_pseudo_tokens(iter, env, Some(ptoken.span.0), None)?;
            let symbol = if inner_ptokens.len() != 0 {
                parse_expr(&inner_ptokens, env)?
            } else {
                Symbol::Nil
            };
            let span = inner_ptokens.as_slice().get_opt_span().unwrap_or((ptoken.span.0, ptoken.span.1));
            PToken::from_symbol(symbol, span)
        } else if ptoken.is_reserved(")") {
            return if start_paren.is_some() {
                Ok(ptokens)
            } else {
                Err(fail_at!(ptoken.span, "unmatched close parenthesis"))
            };
        } else if ptoken.is_reserved("[") {
            // recursive call needs to remember where the brackets started.
            // forget that you're inside a paren for this scenario:
            // [   (   ]    )
            let inner_ptokens = build_pseudo_tokens(iter, env, None, Some(ptoken.span.0))?;
            println!("inner_ptokens: {:?}", inner_ptokens);
            let symbol = parse_seq(&inner_ptokens, env)?;
            // todo can we get the span from the first paren to the last paren?
            let span = inner_ptokens.as_slice().get_opt_span().unwrap_or(ptoken.span);
            PToken::from_symbol(symbol, span)
        } else if ptoken.is_reserved("]") {
            return if start_brack.is_some() {
                Ok(ptokens)
            } else {
                Err(fail_at!(ptoken.span, "unmatched close_parenthesis"))
            };
        } else {
            ptoken
        };
        ptokens.push(value);
    }
}

/// Parse an assignment statement
/// Return Some(result) if it's definitely an assignment statement
/// Return None if not
fn parse_assignment(ptokens: &[PToken], env: &Env) -> Option<Result<Symbol, Error>> {
    let segments = match parse_delimited_list(ptokens, "=", false) {
        Ok(s) => s,
        Err(e) => return Some(Err(e)),
    };
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
            return Ok(symbol.clone());
        }
    }
    // start scanning for operators, starting from the weakest precedence
    let parse_left = |idx: usize| {
        println!("parsing left: {:?}", &ptokens[..idx]);
        parse_expr(&ptokens[..idx], env)
            .concat_err(fail_at!(ptokens[idx].span, "Could not parse left hand operand of operator"))
    };
    let parse_right = |idx: usize| {
        println!("parsing right: {:?}", &ptokens[idx+1..]);
        parse_expr(&ptokens[idx+1..], env)
            .concat_err(fail_at!(ptokens[idx].span, "Could not parse right hand operand of operator"))
    };

    // operator >> (weak function application)
    // right associative, and weakest precedence
    if let Some(idx) = ptokens.iter().position(|ptok| ptok.is_reserved(">>")) {
        println!("found operator >> at idx {}", idx);
        return Ok(Symbol::Apply{args: vec![parse_left(idx)?], target: parse_right(idx)?.into_boxed()});
    }
    // operator +, -
    // left associative
    if let Some(idx) = ptokens.iter().rposition(|ptok| ptok.is_reserved("+") || ptok.is_reserved("-")) {
        println!("found operator +- at idx {}", idx);
        if ptokens[idx].is_reserved("+") {
            return Ok(Symbol::Apply {target: Box::new("add".to_string().into()), args: parse_either_side(ptokens, env, idx)? });
        }
        if ptokens[idx].is_reserved("-") {
            return Ok(Symbol::Apply {target: Box::new("sub".to_string().into()), args: parse_either_side(ptokens, env, idx)? });
        }
    }
    // operator *, /
    // left associative
    if let Some(idx) = ptokens.iter().rposition(|ptok| ptok.is_reserved("*") || ptok.is_reserved("/")) {
        if ptokens[idx].is_reserved("*") {
            return Ok(Symbol::Apply {target: Box::new("mul".to_string().into()), args: parse_either_side(ptokens, env, idx)? });
        }

        if ptokens[idx].is_reserved("/") {
            if idx == 0 && idx != ptokens.len() {
                // missing Left arg and but do have Right arg (which was supplied first)
                return Ok(Symbol::Fn {
                    ptr: Box::new(|mut args| {
                        args.reverse();
                        Symbol::Apply {
                            target: Box::new("div".to_string().into()), args
                        }
                    }),
                    type_: fn_type!(Type::Distr, -> Type::Distr),
                    exprs: vec![parse_right(idx)?]
                });
            }
            return Ok(Symbol::Apply {target: Box::new("div".to_string().into()), args: parse_either_side(ptokens, env, idx)? });
        }
    }
    // strong function application
    if let Some(symbol) = ptokens[0].try_to_expr() {
        let target = symbol.clone().into_boxed();
        let args = ptokens[1..].iter().map(|ptok| {
            ptok.try_to_expr().map(Symbol::clone).ok_or(fail_at!(ptok.span, "function call to `{}` expected valid argument here", target.repr()))
        }).collect::<Result<Vec<Symbol>, Error>>()?;
        return Ok(Symbol::Apply { target, args });
    }

    Err(fail_at!(ptokens.get_opt_span().expect("len 1 and len 0 should be checked"), "could not parse ambiguous expression"))
}

/// parse a sequence literal: `[1, 2, 3]` or `[1; 4]` and the like
fn parse_seq(ptokens: &[PToken], env: &Env) -> Result<Symbol, Error> {
    if ptokens.len() == 0 {
        return Ok(Symbol::Seq(vec![]));
    }
    // todo: parse other seq literals: [d6; 4] and [0..6]
    // parse semi colon syntax: [d6; 4]
    let segments = parse_delimited_list(ptokens, ";", false).concat_err(fail!("could not parse repetition literal"))?;
    match segments.len() {
        2 => return parse_expr(segments[0], env),
        0 | 1 => { } // pass: comma separated list will take care of this
        _ => return Err(fail_at!(ptokens.get_opt_span().unwrap(), "unexpected sequence syntax: too many semicolons")),
    }

    // parse comma separated seq: [3, 4, 5]
    Ok(Symbol::Seq(
        parse_delimited_list(ptokens, ",", true)
            .concat_err(fail!("could not parse as comma separated sequence"))?
            .iter()
            .map(|seg| parse_expr(seg, env))
            .collect::<Result<Vec<Symbol>, Error>>()?
    ))
}

fn parse_delimited_list<'a>(ptokens: &'a [PToken], deliminator: &str, deliminator_can_trail: bool) -> Result<Vec<&'a [PToken]>, Error> {
    if ptokens.is_empty() {
        return Ok(vec![]);
    }
    let segments = ptokens
        .split(|ptoken| ptoken.is_reserved(deliminator))
        .collect::<Vec<&[PToken]>>();
    // if the last segment exists and is empty, we have a trailing deliminator
    let segments =
        if segments.last().expect("expected non-zero ptokens").is_empty() {
            if !deliminator_can_trail {
                return Err(fail_at!(ptokens.last().unwrap().span, "trailing deliminator is not allowed"));
            } else {
                &segments[..segments.len()-1] // slice away the garbage
            }
        } else {
            &segments[..]
        };
    Ok(segments
        .iter()
        .map(|&seg| seg)
        .collect()
    )
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

fn parse_either_side(ptokens: &[PToken], env: &Env, pivot: usize) -> Result<Vec<Symbol>, Error> {
    let mut vec = vec![];
    if !ptokens[..pivot].is_empty() {
        vec.push(parse_expr(&ptokens[..pivot], env)
                     .concat_err(fail_at!(ptokens[pivot].span, "could not parse left hand operand of operator"))?);
    }
    if !ptokens[pivot+1..].is_empty() {
        vec.push(parse_expr(&ptokens[pivot+1..], env)
            .concat_err(fail_at!(ptokens[pivot].span, "could not parse right hand operand of operator"))?);
    }
    Ok(vec)
}