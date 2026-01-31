use std::fmt;

#[derive(Debug, Clone)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}, column {}", self.line, self.column)
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum XeErrorKind {
    // Lexer errors
    UnexpectedCharacter(char),
    UnterminatedString,
    InvalidNumber(String),

    // Parser errors
    UnexpectedToken(String),
    ExpectedToken(String),
    ExpectedExpression,
    ExpectedIdentifier,
    InvalidIndentation,

    // Semantic errors
    UndefinedVariable(String),
    UndefinedFunction(String),
    WrongArgumentCount { name: String, expected: usize, got: usize },
    TypeMismatch { expected: String, got: String },
    CannotRedefineBuiltin(String),

    // General
    IoError(String),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct XeError {
    pub kind: XeErrorKind,
    pub span: Option<Span>,
    pub message: String,
}

impl XeError {
    pub fn new(kind: XeErrorKind, span: Option<Span>) -> Self {
        let message = Self::format_message(&kind);
        Self { kind, span, message }
    }

    fn format_message(kind: &XeErrorKind) -> String {
        match kind {
            XeErrorKind::UnexpectedCharacter(c) => format!("unexpected character '{}'", c),
            XeErrorKind::UnterminatedString => "unterminated string literal".to_string(),
            XeErrorKind::InvalidNumber(s) => format!("invalid number '{}'", s),
            XeErrorKind::UnexpectedToken(t) => format!("unexpected token '{}'", t),
            XeErrorKind::ExpectedToken(t) => format!("expected '{}'", t),
            XeErrorKind::ExpectedExpression => "expected expression".to_string(),
            XeErrorKind::ExpectedIdentifier => "expected identifier".to_string(),
            XeErrorKind::InvalidIndentation => "invalid indentation".to_string(),
            XeErrorKind::UndefinedVariable(name) => format!("undefined variable '{}'", name),
            XeErrorKind::UndefinedFunction(name) => format!("undefined function '{}'", name),
            XeErrorKind::WrongArgumentCount { name, expected, got } => {
                format!("function '{}' expects {} arguments, got {}", name, expected, got)
            }
            XeErrorKind::TypeMismatch { expected, got } => {
                format!("type mismatch: expected {}, got {}", expected, got)
            }
            XeErrorKind::CannotRedefineBuiltin(name) => {
                format!("cannot redefine built-in function '{}'", name)
            }
            XeErrorKind::IoError(msg) => format!("I/O error: {}", msg),
        }
    }
}

impl fmt::Display for XeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.span {
            Some(span) => write!(f, "Error at {}: {}", span, self.message),
            None => write!(f, "Error: {}", self.message),
        }
    }
}

impl std::error::Error for XeError {}

pub type XeResult<T> = Result<T, XeError>;
