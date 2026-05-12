use std::fmt;
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum XeType {
    Number,
    Text,
    Boolean,
    List(Box<XeType>),
    Void,
    Unknown,
}

impl XeType {
    pub fn is_compatible(&self, other: &XeType) -> bool {
        if self == &XeType::Unknown || other == &XeType::Unknown {
            return true;
        }
        self == other
    }

    pub fn name(&self) -> String {
        self.to_string()
    }

    pub fn to_rust_type(&self) -> String {
        match self {
            XeType::Number => "f64".to_string(),
            XeType::Text => "String".to_string(),
            XeType::Boolean => "bool".to_string(),
            XeType::List(inner) => format!("Vec<{}>", inner.to_rust_type()),
            XeType::Void => "()".to_string(),
            XeType::Unknown => "XeValue".to_string(),
        }
    }
}

impl fmt::Display for XeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XeType::Number => write!(f, "number"),
            XeType::Text => write!(f, "text"),
            XeType::Boolean => write!(f, "boolean"),
            XeType::List(inner) => write!(f, "list<{}>", inner),
            XeType::Void => write!(f, "void"),
            XeType::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone)]
pub struct ModulePath {
    pub segments: Vec<String>,
}

impl ModulePath {
    pub fn as_string(&self) -> String {
        self.segments.join(".")
    }
}

#[derive(Debug, Clone)]
pub struct Statement {
    pub kind: StatementKind,
    #[allow(dead_code)]
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Import {
        module: ModulePath,
    },
    FromImport {
        module: ModulePath,
        names: Vec<String>,
    },
    Assignment {
        name: String,
        value: Expression,
    },
    If {
        condition: Expression,
        then_block: Vec<Statement>,
        else_block: Option<Vec<Statement>>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    Repeat {
        count: Expression,
        body: Vec<Statement>,
    },
    For {
        variable: String,
        iterable: Expression,
        body: Vec<Statement>,
    },
    FunctionDef {
        name: String,
        params: Vec<String>,
        body: Vec<Statement>,
    },
    Return {
        value: Option<Expression>,
    },
    Break,
    Continue,
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub kind: ExpressionKind,
    #[allow(dead_code)]
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExpressionKind {
    // Literals
    Number(f64),
    String(String),
    Boolean(bool),
    List(Vec<Expression>),

    // Variable
    Identifier(String),

    // Operations
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<Expression>,
    },

    // Function call
    FunctionCall {
        name: String,
        args: Vec<Expression>,
    },

    // Index access: list[index]
    Index {
        object: Box<Expression>,
        index: Box<Expression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,

    // Logical
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Negate,
    Not,
}

impl BinaryOperator {
    pub fn precedence(&self) -> u8 {
        match self {
            BinaryOperator::Or => 1,
            BinaryOperator::And => 2,
            BinaryOperator::Equal
            | BinaryOperator::NotEqual
            | BinaryOperator::Less
            | BinaryOperator::Greater
            | BinaryOperator::LessEqual
            | BinaryOperator::GreaterEqual => 3,
            BinaryOperator::Add | BinaryOperator::Subtract => 4,
            BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo => 5,
        }
    }
}

// --- Typed AST (Typed IR) ---

#[derive(Debug, Clone)]
pub struct TypedProgram {
    pub statements: Vec<TypedStatement>,
}

#[derive(Debug, Clone)]
pub struct TypedStatement {
    pub kind: TypedStatementKind,
    #[allow(dead_code)]
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypedStatementKind {
    Assignment {
        name: String,
        value: TypedExpression,
    },
    If {
        condition: TypedExpression,
        then_block: Vec<TypedStatement>,
        else_block: Option<Vec<TypedStatement>>,
    },
    While {
        condition: TypedExpression,
        body: Vec<TypedStatement>,
    },
    Repeat {
        count: TypedExpression,
        body: Vec<TypedStatement>,
    },
    For {
        variable: String,
        iterable: TypedExpression,
        body: Vec<TypedStatement>,
    },
    FunctionDef {
        name: String,
        params: Vec<(String, XeType)>,
        body: Vec<TypedStatement>,
        return_type: XeType,
    },
    Return {
        value: Option<TypedExpression>,
    },
    Break,
    Continue,
    Expression(TypedExpression),
}

#[derive(Debug, Clone)]
pub struct TypedExpression {
    pub kind: TypedExpressionKind,
    pub ty: XeType,
    #[allow(dead_code)]
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypedExpressionKind {
    // Literals
    Number(f64),
    String(String),
    Boolean(bool),
    List(Vec<TypedExpression>),

    // Variable
    Identifier(String),

    // Operations
    BinaryOp {
        left: Box<TypedExpression>,
        op: BinaryOperator,
        right: Box<TypedExpression>,
    },
    UnaryOp {
        op: UnaryOperator,
        operand: Box<TypedExpression>,
    },

    // Function call
    FunctionCall {
        name: String,
        args: Vec<TypedExpression>,
    },

    // Index access
    Index {
        object: Box<TypedExpression>,
        index: Box<TypedExpression>,
    },

    // Coercion nodes (The "Wrap/Unwrap" nodes)
    Wrap(Box<TypedExpression>),           // Native -> XeValue (Dynamic)
    Unwrap(Box<TypedExpression>, XeType), // XeValue (Dynamic) -> Native
}
