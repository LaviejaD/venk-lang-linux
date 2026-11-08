use std::fmt;
use crate::lexer::Span;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Severity {
    Error,   // Красный
    Warning, // Желтый
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn error(span: Span, msg: impl Into<String>) -> Self {
        Diagnostic { severity: Severity::Error, span, message: msg.into() }
    }

    pub fn warning(span: Span, msg: impl Into<String>) -> Self {
        Diagnostic { severity: Severity::Warning, span, message: msg.into() }
    }

    /// Форматированный вывод с цветом: line:col и сообщение
    pub fn render(&self) -> String {
        let color = match self.severity {
            Severity::Error => "\x1b[1;31m",   // Ярко-красный
            Severity::Warning => "\x1b[1;33m", // Желтый
        };
        let reset = "\x1b[0m";

        format!(
            "{}{:?} at {}:{}: {}{}",
            color,
            self.severity,
            self.span.line,
            self.span.col,
            self.message,
            reset
        )
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.render())
    }
}

/// Коллектор ошибок (собирает все ошибки за один проход)
#[derive(Default)]
pub struct Diagnostics {
    pub items: Vec<Diagnostic>,
    pub error_count: usize,
    pub warning_count: usize,
}

impl Diagnostics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push_error(&mut self, span: Span, msg: impl Into<String>) {
        self.items.push(Diagnostic::error(span, msg));
        self.error_count += 1;
    }

    pub fn push_warning(&mut self, span: Span, msg: impl Into<String>) {
        self.items.push(Diagnostic::warning(span, msg));
        self.warning_count += 1;
    }

    pub fn print_all(&self) {
        for d in &self.items {
            eprintln!("{}", d.render());
        }
    }

    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }
}