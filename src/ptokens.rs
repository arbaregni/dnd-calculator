use crate::symbols::Symbol;
use crate::distr::KeyType;
use crate::error::Error;
use regex::{Regex, Captures, Match};

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
lazy_static! {
    static ref TOKEN_REGEX: Regex = Regex::new(r"(?x)
     ^\s*(?:
      (?P<ident>[a-zA-Z_][a-zA-Z\-_]*)  # an identifier
     |(?P<dice>(?P<dice_num>[0-9]+)?d(?P<dice_size>[0-9]+))  # a dice literal
     |(?P<num>[0-9]+)          # a numeral
     |(?P<res>>>|[^\s])        # a reserved ptoken
     )").expect("tokenization regex did not compile");
}
pub struct TokenCaptureStream<'a> {
    source: &'a str,
    idx: usize,
}
impl <'a> TokenCaptureStream<'a> {
    pub fn new(source: &'a str) -> Self {
        TokenCaptureStream {
            source,
            idx: 0,
        }
    }
}
impl <'a> std::iter::Iterator for TokenCaptureStream<'a> {
    type Item = Result<PToken, Error>;
    fn next(&mut self) -> Option<Self::Item> {
        TOKEN_REGEX
            .captures(&self.source[self.idx..])
            .and_then(|captures: Captures| {
                if let Some(mat) = captures.get(0) {
                    let result = PToken::from_captures(captures, self.idx);
                    // the match end is the length of the source than we consume
                    self.idx += mat.end();
                    Some(result)
                } else {
                    // if we can't capture anything: it's hopefully just whitespace: so we quit
                    None
                }
            })
    }
}
/// convert the match and where the capture started in the source string into the proper span
fn to_span(mat: Match<'_>, idx: usize) -> (usize, usize) {
    (idx + mat.start(), idx + mat.end())
}
impl PToken {
    fn from_captures(captures: regex::Captures<'_>, idx: usize) -> Result<PToken, Error> {
        let span = (captures.get(0).unwrap().start(), captures.get(0).unwrap().end());
        Ok(if let Some(mat) = captures.name("ident") {
            PToken {
                kind: PTokenKind::Expr(mat.as_str().to_string().into()),
                span: to_span(mat, idx)
            }
        } else if let Some(mat) = captures.name("dice") {
            let dice_num: KeyType = captures.name("dice_num").map_or(Ok(1), |num_mat| {
                println!("{}", num_mat.as_str());
                num_mat.as_str().parse::<KeyType>().map_err(|_| fail_at!(to_span(num_mat, idx), "could not parse dice number specifier in literal"))
            })?;
            let dice_size: KeyType = captures.name("dice_size").map_or(Err(fail_at!(to_span(mat, idx), "dice literal missing dice size specifier")), |size_mat| {
                println!("{}", size_mat.as_str());
                size_mat.as_str().parse::<KeyType>().map_err(|_| fail_at!(to_span(size_mat, idx), "could not parse dice size specifier in literal"))
            })?;
            PToken {
                kind: PTokenKind::Expr(Symbol::Apply { target: Box::new("make-dice".to_string().into()), args: vec![dice_num.into(), dice_size.into()] }),
                span: to_span(mat, idx)
            }
        } else if let Some(mat) = captures.name("res") {
            PToken {
                kind: PTokenKind::Reserved(mat.as_str().to_string().into()),
                span: to_span(mat, idx)
            }
        } else if let Some(mat) = captures.name("num") {
            let num = mat
                .as_str()
                .parse::<KeyType>()
                .map_err(|err| create_err!(err.to_string(), Some(span)))?;
            PToken {
                kind: PTokenKind::Expr(num.into()),
                span: to_span(mat, idx)
            }
        } else {
            return Err(fail_at!(span, "could not construct token"));
        })
    }
    /// creates a PToken from the first and last pseudo tokens that were parsed into the resulting symbol
    pub fn from_symbol(symbol: Symbol, span: (usize, usize)) -> PToken {
        PToken {
            kind: PTokenKind::Expr(symbol),
            span,
        }
    }
    /// returns true if and only if `self` is a PTokenKind::Reserved with that keyword
    ///
    /// # Example
    /// ```
    /// let comma = PToken::from(",", (0, 1));
    /// let dot = PToken::from(".", (0, 1));
    /// let expr = PToken::from_symbol(Symbol::Num(27720), (5, 10));
    /// assert_eq!(true, comma.is_reserved(",");
    /// assert_eq!(false, dot.is_reserved(",");
    /// assert_eq!(false, expr.is_reserved(",")
    /// ```
    pub fn is_reserved(&self, keyword: &str) -> bool {
        self.try_to_reserved().map_or(false, |my_key| my_key == keyword)
    }
    pub fn try_to_reserved(&self) -> Option<&str> {
        match self.kind {
            PTokenKind::Reserved(ref k) => Some(k),
            PTokenKind::Expr(_) => None,
        }
    }
    pub fn try_to_expr(&self) -> Option<&Symbol> {
        match self.kind {
            PTokenKind::Reserved(_) => None,
            PTokenKind::Expr(ref symbol) => Some(symbol),
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