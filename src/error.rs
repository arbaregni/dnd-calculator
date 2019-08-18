// instantiate
macro_rules! fail {
    ($reason_template:expr $(, $arg:expr)* ) => (
        Error {
            tag: String::new(),
            reason: format!($reason_template, $($arg),*),
            opt_span: None,
            line: line!(),
            column: column!(),
            file: file!()
        }
    )
}
macro_rules! fail_at {
   ($span:expr, $reason_template:expr $(, $arg:expr)* ) => (
        Error {
            tag: String::new(),
            reason: format!($reason_template, $($arg),*),
            opt_span: Some($span),
            line: line!(),
            column: column!(),
            file: file!()
        }
    );
}

#[derive(Clone, Debug)]
pub struct Error {
    pub tag: String,
    pub reason: String,
    pub opt_span: Option<(usize, usize)>,
    pub line: u32,
    pub column: u32,
    pub file: &'static str,
}
impl Error {
    /// underline the span in the source string
    pub fn underline(src: &str, span: (usize, usize)) -> String {
        use std::iter;
        format!("{}\n{}{}",
                src,
                iter::repeat(' ').take(span.0).collect::<String>(),
                iter::repeat('^').take(span.1-span.0).collect::<String>())
    }
    /// take an error higher on the error chain, and concatenate it to the one below it
    pub fn concat(self, other: Error) -> Error {
        Error {
            tag: self.tag,
            opt_span: other.opt_span.or(self.opt_span), // attempt to show the lower error, or show the location of the higher level error otherwise
            reason: format!("{}{}", self.reason, format!("\nCaused by: {}", other).replace("\n", "\n    ") ),
            line: self.line,
            column: self.column,
            file: self.file,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "  {}[line:{} col:{} file:{}] {}", self.tag, self.line, self.column, self.file, self.reason)
    }
}