// instantiate the Error
macro_rules! fail {
    ($reason_template:expr $(, $arg:expr)* ) => (
        Error {
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
    /// Concatenate a general error onto this, lower-level situation
    /// # Example
    /// ```
    /// let specific_error = Error {
    ///     opt_span: Some((1, 10)),
    ///     line: 99,
    ///     column: 0,
    ///     file: "file".to_string(),
    ///     reason: "the doo dad didn't work"
    /// };
    /// let general_error = Error {
    ///     opt_span: Some((7, 12)),
    ///     line: 0,
    ///     column: 1,
    ///     file: "general-file".to_string(),
    ///     reason: "parser failed"
    /// };
    /// let new_error = specific_error.concat(general_error);
    /// assert_eq!(new_error, Error {
    ///     // the span is taken from the specific error, or the general error if no span information
    ///     opt_span: Some((1, 10)),
    ///     // the line, column, and file, are taken from the specific error
    ///     line: 99,
    ///     column: 0,
    ///     file: "file".to_string(),
    ///     // call back information is preserved in the reason
    ///     reason: format!("  [line:0 col:1 file:general-file] parser failed\n    Caused by:   [line:99 col:0 file:file] the doo dad didn't work")
    /// })
    /// ```
    pub fn concat(self, general_error: Error) -> Error {
        Error {
            opt_span: self.opt_span.or(general_error.opt_span), // attempt to show the lower error, or show the location of the higher level error otherwise
            reason: format!("{}{}", general_error.reason, format!("\nCaused by: {}", self).replace("\n", "\n    ") ),
            line: self.line,
            column: self.column,
            file: self.file,
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "  [line:{} col:{} file:{}] {}", self.line, self.column, self.file, self.reason)
    }
}

/// A trait to enable us to concate additional error messages to any Result that we return
pub trait ConcatErr {
    type OkType;
    fn concat_err(self, err: Error) -> Result<Self::OkType, Error>;
}
impl<T> ConcatErr for Result<T, Error> {
    type OkType = T;
    fn concat_err(self, err: Error) -> Result<Self::OkType, Error> {
        self.map_err(|prev_err| prev_err.concat(err))
    }
}