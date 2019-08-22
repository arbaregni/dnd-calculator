use crate::symbols::Symbol;
use crate::distr::KeyType;
use crate::error::Error;
use std::any::Any;

#[derive(Debug)]
pub struct PToken {
    pub kind: PTokenKind,
    pub span: (usize, usize),
}
#[derive(Debug)]
pub enum PTokenKind {
    Reserved(String),
    Expr(Symbol),
}
impl PToken {
    /// create a pseudo token from a String and the span of the source where it was taken from
    /// if possible, it parses `raw` into Symbol::Num
    /// if possible, it creates a reserved PTokenKind
    /// other wise, it creates a Symbol::Text
    pub fn from(raw: String, span: (usize, usize)) -> Result<PToken, Error> {
        Ok(PToken {
            kind: match raw.as_str() {
                "*" | "/" | "+" | "-" | "d" | ">>" | "(" | ")" | "[" | "]" | "," | ";" | "=" => PTokenKind::Reserved(raw),
                _ => if raw.chars().nth(0).map_or(false, char::is_numeric) {
                    match raw.parse::<KeyType>() {
                        Ok(num) => PTokenKind::Expr(num.into()),
                        Err(err) => return Err(fail_at!(span, "Could not parse as integer: {}", err)),
                    }
                } else {
                    PTokenKind::Expr(raw.into())
                }
            },
            span
        })
    }
    /// creates a PToken from the first and last pseudo tokens that were parsed into the resulting symbol
    pub fn from_symbol(symbol: Symbol, span: (usize, usize)) -> PToken {
        PToken {
            kind: PTokenKind::Expr(symbol),
            span,
        }
    }
    pub fn try_to_reserved(&self) -> Option<&str> {
        match self.kind {
            PTokenKind::Reserved(ref k) => Some(k),
            PTokenKind::Expr(_) => None,
        }
    }
    pub fn try_to_expr(&self) -> Option<Symbol> {
        match self.kind {
            PTokenKind::Reserved(_) => None,
            PTokenKind::Expr(ref symbol) => Some(symbol.clone()),
        }
    }
}
pub trait MaybeHasSpan {
    fn get_opt_span(&self) -> Option<(usize, usize)>;
}
impl<'a> MaybeHasSpan for &'a [PToken] {
    /// get the span from the first ptoken to the last
    /// or None if there aren't any ptokens
    fn get_opt_span(&self) -> Option<(usize, usize)> {
        Some((self.get(0)?.span.0, self.last()?.span.1))
    }
}