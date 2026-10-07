use std::fmt;
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum XeType {
    Number,
    Text,
    Boolean,
    List(Box<XeType>),
    Map,
    Struct(String),
    Void,
    Unknown,
}

impl XeType {
    pub fn is_compatible(&self, other: &XeType) -> bool {
        match (self, other) {
            (XeType::Unknown, _) | (_, XeType::Unknown) => true,
            (XeType::List(a), XeType::List(b)) => a.is_compatible(b),
            _ => self == other,
        }
    }

    /// True when the Rust representation of this type is the dynamic `XeValue`.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, XeType::Unknown | XeType::Map | XeType::Struct(_))
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
            XeType::Map => "XeValue".to_string(),
            XeType::Struct(_) => "XeValue".to_string(),
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
            XeType::Map => write!(f, "map"),
            XeType::Struct(name) => write!(f, "struct {}", name),
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
    Global {
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
    StructDef {
        name: String,
        fields: Vec<String>,
    },
    IndexAssignment {
        object: Expression,
        index: Expression,
        value: Expression,
    },
    FieldAssignment {
        object: Expression,
        field: String,
        value: Expression,
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
    Map(Vec<(Expression, Expression)>),

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

    // Index access: list[index] or map[key]
    Index {
        object: Box<Expression>,
        index: Box<Expression>,
    },

    // Field access: obj.field
    FieldAccess {
        object: Box<Expression>,
        field: String,
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
    /// Module-level variables and their storage types.
    pub globals: Vec<(String, XeType)>,
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
    StructDef {
        name: String,
        fields: Vec<String>,
    },
    IndexAssignment {
        object: TypedExpression,
        index: TypedExpression,
        value: TypedExpression,
    },
    FieldAssignment {
        object: TypedExpression,
        field: String,
        value: TypedExpression,
    },
    Return {
        value: Option<TypedExpression>,
    },
    Break,
    Continue,
    Expression(TypedExpression),
    Nop,
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
    Map(Vec<(TypedExpression, TypedExpression)>),

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

    // Field access
    FieldAccess {
        object: Box<TypedExpression>,
        field: String,
    },

    // Coercion nodes (The "Wrap/Unwrap" nodes)
    Wrap(Box<TypedExpression>),           // Native -> XeValue (Dynamic)
    Unwrap(Box<TypedExpression>, XeType), // Any type -> the given type (runtime-checked)
}

/// Whether executing this block is guaranteed to hit a `return`.
pub fn block_always_returns(statements: &[Statement]) -> bool {
    statements.iter().any(|statement| match &statement.kind {
        StatementKind::Return { .. } => true,
        StatementKind::If {
            then_block,
            else_block: Some(else_block),
            ..
        } => block_always_returns(then_block) && block_always_returns(else_block),
        _ => false,
    })
}
