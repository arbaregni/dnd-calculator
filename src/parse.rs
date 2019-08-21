use crate::symbols::Symbol;
use crate::distr::{KeyType};
use crate::operations::Op;
use crate::env::Env;
use crate::error::Error;
use crate::error::ConcatErr;

use regex::Regex;

#[derive(Debug)]
struct PToken {
    kind: PTokenKind,
    span: (usize, usize),
}
#[derive(Debug)]
enum PTokenKind {
    Reserved(String),
    Expr(Symbol),
}
impl PToken {
    /// create a pseudo token from a String and the span of the source where it was taken from
    /// if possible, it parses `raw` into Symbol::Num
    /// if possible, it creates a reserved PTokenKind
    /// other wise, it creates a Symbol::Text
    fn from(raw: String, span: (usize, usize)) -> PToken {
        PToken {
            kind: match raw.parse::<KeyType>() {
                Ok(num) => PTokenKind::Expr(num.into()),
                Err(_)  => match raw.as_str() { // todo only catch errors due to incorrect digits
                    "*" | "/" | "+" | "-" | "d" | ">>" | "(" | ")" | "[" | "]" | "," | ";" => PTokenKind::Reserved(raw),
                    _ => PTokenKind::Expr(raw.into())
                }
            },
            span
        }
    }
    /// creates a PToken from the first and last pseudo tokens that were parsed into the resulting symbol
    fn from_compound(symbol: Symbol, inner_ptokens: &[PToken]) -> PToken {
        PToken {
            kind: PTokenKind::Expr(symbol),
            span: (inner_ptokens.get(0).expect("can't handle zero sized inner_ptokens yet").span.0, inner_ptokens.last().expect("zero sized inner_ptokens not handled yet").span.1) // expand to include the entire expression that produced the symbol
        }
    }
    fn try_to_reserved(&self) -> Option<&str> {
        match self.kind {
            PTokenKind::Reserved(ref k) => Some(k),
            PTokenKind::Expr(_) => None,
        }
    }
    fn try_to_expr(&self) -> Option<Symbol> {
        match self.kind {
            PTokenKind::Reserved(_) => None,
            PTokenKind::Expr(ref symbol) => Some(symbol.clone()),
        }
    }
}
pub fn parse_line(src: &str, env: &Env) -> Result<Symbol, Error> {
    let mut iter = tokenize(src);
    let ptokens = build_pseudo_tokens(&mut iter, env, None, None)?;
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
                let symbol = parse_expr(&inner_ptokens, env)?;
                PToken::from_compound(symbol, &inner_ptokens)
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
                PToken::from_compound(symbol, &inner_ptokens)
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
        None => return Err(fail_at!((ptokens.get(0).unwrap().span.0, ptokens.last().unwrap().span.1), "no operator found here"))
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

/// parse a sequence literal
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
        _ => return Err(fail_at!((ptokens[0].span.0, ptokens[ptokens.len()-1].span.1), "unexpected sequence syntax: too many semicolons")),
    }

    // parse comma separated seq: [3, 4, 5]
    let vec = ptokens
        .split(|ptoken| ptoken.try_to_reserved().map_or(false, |k| k == ","))
        .map(|segment| parse_expr(segment, env).concat_err(fail!("could not parse as comma separated sequence")))
        .collect::<Result<Vec<Symbol>, Error>>()?;
    Ok(Symbol::Seq(vec))
}
