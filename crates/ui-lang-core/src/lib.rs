mod ast;
mod check;
mod codegen;
mod format;
mod parser;
mod source;
#[cfg(test)]
mod test_support;

pub use ast::*;
pub use format::{format_fragment, format_source};
pub use source::{
    FileCompilation, analyze_file, analyze_file_with_source, compile_file, source_is_app,
};

use std::fmt;
use std::ops::Deref;

#[derive(Clone, Debug)]
pub struct CheckedDocument(Document);

impl CheckedDocument {
    pub(crate) fn new(document: Document) -> Self {
        Self(document)
    }
}

impl Deref for CheckedDocument {
    type Target = Document;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

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
        if let Ok(source) = std::fs::read_to_string(path)
            && let Some(index) = self.line.checked_sub(1)
            && let Some(line) = source.lines().nth(index)
        {
            let gutter = self.line.to_string();
            let column = self.column.saturating_sub(1).min(line.chars().count());
            rendered.push_str(&format!(
                "\n{gutter} | {line}\n{} | {}^",
                " ".repeat(gutter.len()),
                " ".repeat(column)
            ));
        }
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

pub fn analyze(source: &str) -> Result<CheckedDocument, Error> {
    check::analyze(parse(source)?)
}

pub fn compile(source: &str, source_path: &str) -> Result<String, Error> {
    let document = analyze(source)?;
    codegen::generate(&document, source_path)
}
