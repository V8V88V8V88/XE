use std::fmt;
use crate::builtins::Builtin;
use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum XeType {
    Number,
    Text,
    Boolean,
    /// Lists, maps, structs and functions are shared references to dynamic values.
    List,
    Map,
    Struct(String),
    Function,
    Void,
    Unknown,
}

impl XeType {
    pub fn is_compatible(&self, other: &XeType) -> bool {
        match (self, other) {
            (XeType::Unknown, _) | (_, XeType::Unknown) => true,
            _ => self == other,
        }
    }

    /// True when the Rust representation of this type is the dynamic `XeValue`.
    pub fn is_dynamic(&self) -> bool {
        matches!(
            self,
            XeType::Unknown | XeType::List | XeType::Map | XeType::Struct(_) | XeType::Function
        )
    }

    /// Number, text and boolean have native (unboxed) Rust representations.
    pub fn is_scalar(&self) -> bool {
        matches!(self, XeType::Number | XeType::Text | XeType::Boolean)
    }

    pub fn name(&self) -> String {
        self.to_string()
    }

    pub fn to_rust_type(&self) -> String {
        match self {
            XeType::Number => "f64".to_string(),
            XeType::Text => "String".to_string(),
            XeType::Boolean => "bool".to_string(),
            XeType::Void => "()".to_string(),
            _ => "XeValue".to_string(),
        }
    }
}

impl fmt::Display for XeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XeType::Number => write!(f, "number"),
            XeType::Text => write!(f, "text"),
            XeType::Boolean => write!(f, "boolean"),
            XeType::List => write!(f, "list"),
            XeType::Map => write!(f, "map"),
            XeType::Struct(name) => write!(f, "struct {}", name),
            XeType::Function => write!(f, "function"),
            XeType::Void => write!(f, "none"),
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
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum StatementKind {
    Import {
        module: ModulePath,
        alias: Option<String>,
    },
    FromImport {
        module: ModulePath,
        /// (imported name, local name)
        names: Vec<(String, String)>,
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
    /// `object[index] = value`, or `object[index] op= value` when `op` is set.
    IndexAssignment {
        object: Expression,
        index: Expression,
        value: Expression,
        op: Option<BinaryOperator>,
    },
    /// `object.field = value`, or `object.field op= value` when `op` is set.
    FieldAssignment {
        object: Expression,
        field: String,
        value: Expression,
        op: Option<BinaryOperator>,
    },
    Try {
        body: Vec<Statement>,
        catch_variable: Option<String>,
        handler: Vec<Statement>,
    },
    Return {
        value: Option<Expression>,
    },
    Break,
    Continue,
    Pass,
    Expression(Expression),
}

#[derive(Debug, Clone)]
pub struct Expression {
    pub kind: ExpressionKind,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ExpressionKind {
    // Literals
    Number(f64),
    String(String),
    Boolean(bool),
    None,
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

    /// Call of a named function or of a variable holding a function: `name(args)`.
    FunctionCall {
        name: String,
        args: Vec<Expression>,
    },
    /// `object.method(args)`: a module function, a function called with `object` as its
    /// first argument, or a function stored in a field.
    MethodCall {
        object: Box<Expression>,
        method: String,
        args: Vec<Expression>,
    },
    /// Call of any other expression, e.g. `make_adder(1)(2)`.
    Call {
        callee: Box<Expression>,
        args: Vec<Expression>,
    },
    Lambda {
        params: Vec<String>,
        body: Box<Expression>,
    },

    // Index access: list[index] or map[key]
    Index {
        object: Box<Expression>,
        index: Box<Expression>,
    },
    Slice {
        object: Box<Expression>,
        start: Option<Box<Expression>>,
        end: Option<Box<Expression>>,
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
    FloorDivide,
    Modulo,
    Power,

    // Comparison
    Equal,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    In,
    NotIn,

    // Logical
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOperator {
    Negate,
    Not,
}

/// Precedence of the prefix `not` operator: below comparisons, above `and`.
pub const NOT_PRECEDENCE: u8 = 3;
/// Precedence of unary minus: below `**`, above `*`.
pub const NEGATE_PRECEDENCE: u8 = 7;

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
            | BinaryOperator::GreaterEqual
            | BinaryOperator::In
            | BinaryOperator::NotIn => 4,
            BinaryOperator::Add | BinaryOperator::Subtract => 5,
            BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::FloorDivide
            | BinaryOperator::Modulo => 6,
            BinaryOperator::Power => 8,
        }
    }

    pub fn is_right_associative(&self) -> bool {
        matches!(self, BinaryOperator::Power)
    }

    pub fn symbol(&self) -> &'static str {
        match self {
            BinaryOperator::Add => "+",
            BinaryOperator::Subtract => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::FloorDivide => "//",
            BinaryOperator::Modulo => "%",
            BinaryOperator::Power => "**",
            BinaryOperator::Equal => "==",
            BinaryOperator::NotEqual => "!=",
            BinaryOperator::Less => "<",
            BinaryOperator::Greater => ">",
            BinaryOperator::LessEqual => "<=",
            BinaryOperator::GreaterEqual => ">=",
            BinaryOperator::In => "in",
            BinaryOperator::NotIn => "not in",
            BinaryOperator::And => "and",
            BinaryOperator::Or => "or",
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
        variable_type: XeType,
        iterable: TypedExpression,
        body: Vec<TypedStatement>,
    },
    FunctionDef {
        name: String,
        params: Vec<(String, XeType)>,
        /// Variables assigned in the body (excluding parameters and loop variables).
        locals: Vec<(String, XeType)>,
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
        op: Option<BinaryOperator>,
    },
    FieldAssignment {
        object: TypedExpression,
        field: String,
        value: TypedExpression,
        op: Option<BinaryOperator>,
    },
    Try {
        body: Vec<TypedStatement>,
        catch_variable: Option<String>,
        handler: Vec<TypedStatement>,
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
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum TypedExpressionKind {
    // Literals
    Number(f64),
    String(String),
    Boolean(bool),
    None,
    List(Vec<TypedExpression>),
    Map(Vec<(TypedExpression, TypedExpression)>),

    // Variable
    Identifier(String),
    /// A user function or struct constructor used as a value.
    FunctionRef(String),
    /// A built-in function used as a value.
    BuiltinRef(Builtin),

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

    /// Direct call of a user function or struct constructor.
    FunctionCall {
        name: String,
        args: Vec<TypedExpression>,
    },
    BuiltinCall {
        builtin: Builtin,
        args: Vec<TypedExpression>,
    },
    /// Call of a function value; checked at runtime.
    DynamicCall {
        callee: Box<TypedExpression>,
        args: Vec<TypedExpression>,
    },
    Lambda {
        params: Vec<String>,
        /// Enclosing local variables the body uses, copied when the lambda is created.
        captures: Vec<(String, XeType)>,
        body: Box<TypedExpression>,
    },

    // Index access
    Index {
        object: Box<TypedExpression>,
        index: Box<TypedExpression>,
    },
    Slice {
        object: Box<TypedExpression>,
        start: Option<Box<TypedExpression>>,
        end: Option<Box<TypedExpression>>,
    },

    // Field access
    FieldAccess {
        object: Box<TypedExpression>,
        field: String,
    },

    /// The truthiness of any value, used for conditions.
    Truthy(Box<TypedExpression>),
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
        StatementKind::Try { body, handler, .. } => {
            block_always_returns(body) && block_always_returns(handler)
        }
        _ => false,
    })
}
