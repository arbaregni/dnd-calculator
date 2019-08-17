// instantiate
macro_rules! fail {
    ($reason_template:expr $(, $arg:expr)* ) => (
        Error {
            tag: String::new(),
            reason: format!($reason_template, $($arg),*),
            line: line!(),
            column: column!(),
            file: file!()
        }
    )
}
macro_rules! enrich {
    ($err:expr, $info_template:expr $(, $arg:expr)* ) => (
        fail!($info_template, $($arg),*).concat($err)
    )
}

#[derive(Clone, Debug)]
pub struct Error {
    pub tag: String,
    pub reason: String,
    pub line: u32,
    pub column: u32,
    pub file: &'static str,
}
impl Error {
    /// take an error higher on the error chain, and concatenate it to the one below it
    pub fn concat(self, other: Error) -> Error {
        Error {
            tag: self.tag,
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