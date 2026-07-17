mod ast;
mod check;
mod codegen;
mod format;
mod parser;

pub use ast::*;
pub use format::format_source;

use std::fmt;

#[derive(Clone, Debug)]
pub struct Error {
    pub code: &'static str,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub hint: Option<String>,
}

impl Error {
    pub(crate) fn new(code: &'static str, span: &Span, message: impl Into<String>) -> Self {
        Self {
            code,
            line: span.line,
            column: span.column,
            message: message.into(),
            hint: None,
        }
    }

    pub(crate) fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn render(&self, path: &str) -> String {
        let mut rendered = format!(
            "{} {}:{}:{}: {}",
            self.code, path, self.line, self.column, self.message
        );
        if let Some(hint) = &self.hint {
            rendered.push_str("\nhint: ");
            rendered.push_str(hint);
        }
        rendered
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}: {}",
            self.code, self.line, self.column, self.message
        )
    }
}

impl std::error::Error for Error {}

pub fn parse(source: &str) -> Result<Document, Error> {
    parser::parse(source)
}

pub fn analyze(source: &str) -> Result<Document, Error> {
    let mut document = parse(source)?;
    check::check(&mut document)?;
    Ok(document)
}

pub fn compile(source: &str, source_path: &str) -> Result<String, Error> {
    let document = analyze(source)?;
    codegen::generate(&document, source_path)
}
