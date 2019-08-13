macro_rules! fail {
    ($reason_template:expr $(, $arg:expr)* ) => (
        Error{
            tag: String::new(),
            reason: format!($reason_template, $($arg),*),
            line: line!(),
            column: column!(),
            file: file!()
        }
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
    pub fn consolidate(errs: &[Error]) -> Error {
        errs[0].clone()
    }
    pub fn pretty_print(&self) {
        println!("  [{}] {}", self.tag, self.reason);
        println!("  line:{} column:{} ({})", self.line, self.column, self.file);
    }
}