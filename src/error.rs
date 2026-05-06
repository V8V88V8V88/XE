use std::fmt;

#[derive(Debug, Clone)]
pub struct Span {
    pub line: usize,
    pub column: usize,
    pub source_name: Option<String>,
}

impl Span {
    pub fn new(line: usize, column: usize) -> Self {
        Self {
            line,
            column,
            source_name: None,
        }
    }

    pub fn with_source(line: usize, column: usize, source_name: impl Into<String>) -> Self {
        Self {
            line,
            column,
            source_name: Some(source_name.into()),
        }
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
    WrongArgumentCount {
        name: String,
        expected: usize,
        got: usize,
    },
    TypeMismatch {
        expected: String,
        got: String,
    },
    CannotRedefineBuiltin(String),
    DuplicateFunction(String),
    ReturnOutsideFunction,
    BreakOutsideLoop,
    ContinueOutsideLoop,
    ModuleNotFound(String),
    CircularImport(String),
    ImportNotTopLevel,
    ImportAfterExecutableStatement,
    ImportedNameNotFound {
        module: String,
        name: String,
    },
    DuplicateImport(String),
    ImportNameConflict(String),

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
        Self {
            kind,
            span,
            message,
        }
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
            XeErrorKind::WrongArgumentCount {
                name,
                expected,
                got,
            } => {
                format!(
                    "function '{}' expects {} arguments, got {}",
                    name, expected, got
                )
            }
            XeErrorKind::TypeMismatch { expected, got } => {
                format!("type mismatch: expected {}, got {}", expected, got)
            }
            XeErrorKind::CannotRedefineBuiltin(name) => {
                format!("cannot redefine built-in function '{}'", name)
            }
            XeErrorKind::DuplicateFunction(name) => {
                format!("function '{}' is already defined", name)
            }
            XeErrorKind::ReturnOutsideFunction => {
                "return can only be used inside a function".to_string()
            }
            XeErrorKind::BreakOutsideLoop => "break can only be used inside a loop".to_string(),
            XeErrorKind::ContinueOutsideLoop => {
                "continue can only be used inside a loop".to_string()
            }
            XeErrorKind::ModuleNotFound(module) => {
                format!("module '{}' could not be found", module)
            }
            XeErrorKind::CircularImport(path) => {
                format!("circular import detected: {}", path)
            }
            XeErrorKind::ImportNotTopLevel => {
                "import statements are only allowed at the top level".to_string()
            }
            XeErrorKind::ImportAfterExecutableStatement => {
                "import statements must appear before executable top-level statements".to_string()
            }
            XeErrorKind::ImportedNameNotFound { module, name } => {
                format!("module '{}' does not export '{}'", module, name)
            }
            XeErrorKind::DuplicateImport(name) => {
                format!("'{}' is imported more than once", name)
            }
            XeErrorKind::ImportNameConflict(name) => {
                format!(
                    "imported name '{}' conflicts with an existing function",
                    name
                )
            }
            XeErrorKind::IoError(msg) => format!("I/O error: {}", msg),
        }
    }

    pub fn render_with_source(&self, source: &str) -> String {
        match &self.span {
            Some(span) => {
                let line_text = source
                    .lines()
                    .nth(span.line.saturating_sub(1))
                    .unwrap_or_default();
                let caret_column = span.column.saturating_sub(1);
                let caret_padding = " ".repeat(caret_column);
                let header = match &span.source_name {
                    Some(source_name) => {
                        format!("Error in {} at {}: {}", source_name, span, self.message)
                    }
                    None => format!("Error at {}: {}", span, self.message),
                };

                format!("{}\n{}\n{}^", header, line_text, caret_padding)
            }
            None => self.to_string(),
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
