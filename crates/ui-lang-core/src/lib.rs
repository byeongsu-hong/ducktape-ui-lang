mod ast;
mod check;
mod codegen;
mod format;
mod parser;
mod source;

pub use ast::*;
pub use format::{format_fragment, format_source};
pub use source::{FileCompilation, analyze_file, compile_file, source_is_app};

use std::fmt;

#[derive(Clone, Debug)]
pub struct Error {
    pub code: &'static str,
    pub path: Option<String>,
    pub line: usize,
    pub column: usize,
    pub message: String,
    pub hint: Option<String>,
}

impl Error {
    pub(crate) fn new(code: &'static str, span: &Span, message: impl Into<String>) -> Self {
        Self {
            code,
            path: None,
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

    pub(crate) fn at_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn render(&self, path: &str) -> String {
        let path = self.path.as_deref().unwrap_or(path);
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
        let path = self
            .path
            .as_deref()
            .map(|path| format!(" in {path}"))
            .unwrap_or_default();
        write!(
            f,
            "{} at {}:{}{}: {}",
            self.code, self.line, self.column, path, self.message
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
