use crate::ast::*;

pub struct CodeGenerator {
    output: String,
    indent_level: usize,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
        }
    }

    pub fn generate(&mut self, program: &Program) -> String {
        // Generate runtime and prelude
        self.emit_prelude();

        // Collect user-defined functions
        let mut functions = Vec::new();
        let mut main_statements = Vec::new();

        for stmt in &program.statements {
            if matches!(stmt.kind, StatementKind::FunctionDef { .. }) {
                functions.push(stmt);
            } else {
                main_statements.push(stmt);
            }
        }

        // Generate user-defined functions
        for func in functions {
            self.generate_statement(func);
            self.emit("\n");
        }

        // Generate main function
        self.emit("fn main() {\n");
        self.indent_level += 1;

        for stmt in main_statements {
            self.generate_statement(stmt);
        }

        self.indent_level -= 1;
        self.emit("}\n");

        self.output.clone()
    }

    fn emit_prelude(&mut self) {
        self.emit(
            r#"#![allow(dead_code, unused_mut, unused_variables)]
use std::io::{self, Write};

#[derive(Clone, Debug)]
enum XeValue {
    Number(f64),
    Text(String),
    Boolean(bool),
    List(Vec<XeValue>),
}

impl std::fmt::Display for XeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            XeValue::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            XeValue::Text(s) => write!(f, "{}", s),
            XeValue::Boolean(b) => write!(f, "{}", if *b { "true" } else { "false" }),
            XeValue::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
        }
    }
}

impl XeValue {
    fn as_number(&self) -> f64 {
        match self {
            XeValue::Number(n) => *n,
            XeValue::Text(s) => s.parse().unwrap_or(0.0),
            XeValue::Boolean(b) => if *b { 1.0 } else { 0.0 },
            XeValue::List(l) => l.len() as f64,
        }
    }

    fn as_bool(&self) -> bool {
        match self {
            XeValue::Number(n) => *n != 0.0,
            XeValue::Text(s) => !s.is_empty(),
            XeValue::Boolean(b) => *b,
            XeValue::List(l) => !l.is_empty(),
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            XeValue::Number(_) => "number",
            XeValue::Text(_) => "text",
            XeValue::Boolean(_) => "boolean",
            XeValue::List(_) => "list",
        }
    }
}

fn xe_print(args: Vec<XeValue>) {
    let output: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    println!("{}", output.join(" "));
}

fn xe_input(prompt: &XeValue) -> XeValue {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    XeValue::Text(input.trim().to_string())
}

fn xe_length(value: &XeValue) -> XeValue {
    match value {
        XeValue::Text(s) => XeValue::Number(s.len() as f64),
        XeValue::List(l) => XeValue::Number(l.len() as f64),
        _ => XeValue::Number(0.0),
    }
}

fn xe_type(value: &XeValue) -> XeValue {
    XeValue::Text(value.type_name().to_string())
}

fn xe_convert(value: &XeValue, target: &XeValue) -> XeValue {
    let target_type = match target {
        XeValue::Text(s) => s.as_str(),
        _ => return value.clone(),
    };
    match target_type {
        "number" => XeValue::Number(value.as_number()),
        "text" => XeValue::Text(value.to_string()),
        "boolean" => XeValue::Boolean(value.as_bool()),
        _ => value.clone(),
    }
}

fn xe_add(left: XeValue, right: XeValue) -> XeValue {
    match (&left, &right) {
        (XeValue::Text(a), _) => XeValue::Text(format!("{}{}", a, right)),
        (_, XeValue::Text(b)) => XeValue::Text(format!("{}{}", left, b)),
        (XeValue::List(a), XeValue::List(b)) => {
            let mut result = a.clone();
            result.extend(b.clone());
            XeValue::List(result)
        }
        _ => XeValue::Number(left.as_number() + right.as_number()),
    }
}

fn xe_sub(left: XeValue, right: XeValue) -> XeValue {
    XeValue::Number(left.as_number() - right.as_number())
}

fn xe_mul(left: XeValue, right: XeValue) -> XeValue {
    XeValue::Number(left.as_number() * right.as_number())
}

fn xe_div(left: XeValue, right: XeValue) -> XeValue {
    XeValue::Number(left.as_number() / right.as_number())
}

fn xe_mod(left: XeValue, right: XeValue) -> XeValue {
    XeValue::Number(left.as_number() % right.as_number())
}

fn xe_eq(left: &XeValue, right: &XeValue) -> bool {
    match (left, right) {
        (XeValue::Number(a), XeValue::Number(b)) => (a - b).abs() < f64::EPSILON,
        (XeValue::Text(a), XeValue::Text(b)) => a == b,
        (XeValue::Boolean(a), XeValue::Boolean(b)) => a == b,
        _ => false,
    }
}

fn xe_lt(left: &XeValue, right: &XeValue) -> bool {
    left.as_number() < right.as_number()
}

fn xe_gt(left: &XeValue, right: &XeValue) -> bool {
    left.as_number() > right.as_number()
}

fn xe_le(left: &XeValue, right: &XeValue) -> bool {
    left.as_number() <= right.as_number()
}

fn xe_ge(left: &XeValue, right: &XeValue) -> bool {
    left.as_number() >= right.as_number()
}

fn xe_index(obj: &XeValue, idx: &XeValue) -> XeValue {
    let i = idx.as_number() as usize;
    match obj {
        XeValue::List(l) => l.get(i).cloned().unwrap_or(XeValue::Number(0.0)),
        XeValue::Text(s) => {
            s.chars().nth(i).map(|c| XeValue::Text(c.to_string())).unwrap_or(XeValue::Text(String::new()))
        }
        _ => XeValue::Number(0.0),
    }
}

"#,
        );
    }

    fn generate_statement(&mut self, stmt: &Statement) {
        match &stmt.kind {
            StatementKind::Assignment { name, value } => {
                self.emit_indent();
                self.emit(&format!("let mut {} = ", Self::sanitize_name(name)));
                self.generate_expression(value);
                self.emit(";\n");
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                self.emit_indent();
                self.emit("if ");
                self.generate_condition(condition);
                self.emit(" {\n");
                self.indent_level += 1;
                for s in then_block {
                    self.generate_statement(s);
                }
                self.indent_level -= 1;
                self.emit_indent();
                self.emit("}");
                if let Some(else_stmts) = else_block {
                    self.emit(" else {\n");
                    self.indent_level += 1;
                    for s in else_stmts {
                        self.generate_statement(s);
                    }
                    self.indent_level -= 1;
                    self.emit_indent();
                    self.emit("}");
                }
                self.emit("\n");
            }
            StatementKind::Repeat { count, body } => {
                self.emit_indent();
                self.emit("for _ in 0..(");
                self.generate_expression(count);
                self.emit(").as_number() as usize {\n");
                self.indent_level += 1;
                for s in body {
                    self.generate_statement(s);
                }
                self.indent_level -= 1;
                self.emit_indent();
                self.emit("}\n");
            }
            StatementKind::FunctionDef { name, params, body } => {
                self.emit(&format!(
                    "fn {}({}) -> XeValue {{\n",
                    Self::sanitize_name(name),
                    params
                        .iter()
                        .map(|p| format!("{}: XeValue", Self::sanitize_name(p)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                self.indent_level += 1;

                // Make params mutable
                for param in params {
                    self.emit_indent();
                    self.emit(&format!(
                        "let mut {} = {};\n",
                        Self::sanitize_name(param),
                        Self::sanitize_name(param)
                    ));
                }

                for s in body {
                    self.generate_statement(s);
                }

                // Default return
                self.emit_indent();
                self.emit("XeValue::Number(0.0)\n");

                self.indent_level -= 1;
                self.emit("}\n");
            }
            StatementKind::Return { value } => {
                self.emit_indent();
                self.emit("return ");
                if let Some(expr) = value {
                    self.generate_expression(expr);
                } else {
                    self.emit("XeValue::Number(0.0)");
                }
                self.emit(";\n");
            }
            StatementKind::Expression(expr) => {
                self.emit_indent();
                self.generate_expression(expr);
                self.emit(";\n");
            }
        }
    }

    fn generate_condition(&mut self, expr: &Expression) {
        match &expr.kind {
            ExpressionKind::BinaryOp { left, op, right } => match op {
                BinaryOperator::Equal => {
                    self.emit("xe_eq(&");
                    self.generate_expression(left);
                    self.emit(", &");
                    self.generate_expression(right);
                    self.emit(")");
                }
                BinaryOperator::NotEqual => {
                    self.emit("!xe_eq(&");
                    self.generate_expression(left);
                    self.emit(", &");
                    self.generate_expression(right);
                    self.emit(")");
                }
                BinaryOperator::Less => {
                    self.emit("xe_lt(&");
                    self.generate_expression(left);
                    self.emit(", &");
                    self.generate_expression(right);
                    self.emit(")");
                }
                BinaryOperator::Greater => {
                    self.emit("xe_gt(&");
                    self.generate_expression(left);
                    self.emit(", &");
                    self.generate_expression(right);
                    self.emit(")");
                }
                BinaryOperator::LessEqual => {
                    self.emit("xe_le(&");
                    self.generate_expression(left);
                    self.emit(", &");
                    self.generate_expression(right);
                    self.emit(")");
                }
                BinaryOperator::GreaterEqual => {
                    self.emit("xe_ge(&");
                    self.generate_expression(left);
                    self.emit(", &");
                    self.generate_expression(right);
                    self.emit(")");
                }
                BinaryOperator::And => {
                    self.emit("(");
                    self.generate_condition(left);
                    self.emit(" && ");
                    self.generate_condition(right);
                    self.emit(")");
                }
                BinaryOperator::Or => {
                    self.emit("(");
                    self.generate_condition(left);
                    self.emit(" || ");
                    self.generate_condition(right);
                    self.emit(")");
                }
                _ => {
                    // For other binary ops, convert to bool
                    self.emit("(");
                    self.generate_expression(expr);
                    self.emit(").as_bool()");
                }
            },
            ExpressionKind::UnaryOp { op, operand } if *op == UnaryOperator::Not => {
                self.emit("!(");
                self.generate_condition(operand);
                self.emit(")");
            }
            ExpressionKind::Boolean(b) => {
                self.emit(if *b { "true" } else { "false" });
            }
            _ => {
                self.emit("(");
                self.generate_expression(expr);
                self.emit(").as_bool()");
            }
        }
    }

    fn generate_expression(&mut self, expr: &Expression) {
        match &expr.kind {
            ExpressionKind::Number(n) => {
                self.emit(&format!("XeValue::Number({:?})", n));
            }
            ExpressionKind::String(s) => {
                self.emit(&format!("XeValue::Text({:?}.to_string())", s));
            }
            ExpressionKind::Boolean(b) => {
                self.emit(&format!("XeValue::Boolean({})", b));
            }
            ExpressionKind::List(elements) => {
                self.emit("XeValue::List(vec![");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.emit(", ");
                    }
                    self.generate_expression(elem);
                }
                self.emit("])");
            }
            ExpressionKind::Identifier(name) => {
                self.emit(&format!("{}.clone()", Self::sanitize_name(name)));
            }
            ExpressionKind::BinaryOp { left, op, right } => {
                match op {
                    BinaryOperator::Add => {
                        self.emit("xe_add(");
                        self.generate_expression(left);
                        self.emit(", ");
                        self.generate_expression(right);
                        self.emit(")");
                    }
                    BinaryOperator::Subtract => {
                        self.emit("xe_sub(");
                        self.generate_expression(left);
                        self.emit(", ");
                        self.generate_expression(right);
                        self.emit(")");
                    }
                    BinaryOperator::Multiply => {
                        self.emit("xe_mul(");
                        self.generate_expression(left);
                        self.emit(", ");
                        self.generate_expression(right);
                        self.emit(")");
                    }
                    BinaryOperator::Divide => {
                        self.emit("xe_div(");
                        self.generate_expression(left);
                        self.emit(", ");
                        self.generate_expression(right);
                        self.emit(")");
                    }
                    BinaryOperator::Modulo => {
                        self.emit("xe_mod(");
                        self.generate_expression(left);
                        self.emit(", ");
                        self.generate_expression(right);
                        self.emit(")");
                    }
                    BinaryOperator::Equal => {
                        self.emit("XeValue::Boolean(xe_eq(&");
                        self.generate_expression(left);
                        self.emit(", &");
                        self.generate_expression(right);
                        self.emit("))");
                    }
                    BinaryOperator::NotEqual => {
                        self.emit("XeValue::Boolean(!xe_eq(&");
                        self.generate_expression(left);
                        self.emit(", &");
                        self.generate_expression(right);
                        self.emit("))");
                    }
                    BinaryOperator::Less => {
                        self.emit("XeValue::Boolean(xe_lt(&");
                        self.generate_expression(left);
                        self.emit(", &");
                        self.generate_expression(right);
                        self.emit("))");
                    }
                    BinaryOperator::Greater => {
                        self.emit("XeValue::Boolean(xe_gt(&");
                        self.generate_expression(left);
                        self.emit(", &");
                        self.generate_expression(right);
                        self.emit("))");
                    }
                    BinaryOperator::LessEqual => {
                        self.emit("XeValue::Boolean(xe_le(&");
                        self.generate_expression(left);
                        self.emit(", &");
                        self.generate_expression(right);
                        self.emit("))");
                    }
                    BinaryOperator::GreaterEqual => {
                        self.emit("XeValue::Boolean(xe_ge(&");
                        self.generate_expression(left);
                        self.emit(", &");
                        self.generate_expression(right);
                        self.emit("))");
                    }
                    BinaryOperator::And => {
                        self.emit("XeValue::Boolean(");
                        self.generate_condition(left);
                        self.emit(" && ");
                        self.generate_condition(right);
                        self.emit(")");
                    }
                    BinaryOperator::Or => {
                        self.emit("XeValue::Boolean(");
                        self.generate_condition(left);
                        self.emit(" || ");
                        self.generate_condition(right);
                        self.emit(")");
                    }
                }
            }
            ExpressionKind::UnaryOp { op, operand } => match op {
                UnaryOperator::Negate => {
                    self.emit("XeValue::Number(-(");
                    self.generate_expression(operand);
                    self.emit(").as_number())");
                }
                UnaryOperator::Not => {
                    self.emit("XeValue::Boolean(!(");
                    self.generate_expression(operand);
                    self.emit(").as_bool())");
                }
            },
            ExpressionKind::FunctionCall { name, args } => {
                match name.as_str() {
                    "print" => {
                        self.emit("{ xe_print(vec![");
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                self.emit(", ");
                            }
                            self.generate_expression(arg);
                        }
                        self.emit("]); XeValue::Number(0.0) }");
                    }
                    "input" => {
                        self.emit("xe_input(&");
                        if !args.is_empty() {
                            self.generate_expression(&args[0]);
                        } else {
                            self.emit("XeValue::Text(String::new())");
                        }
                        self.emit(")");
                    }
                    "length" => {
                        self.emit("xe_length(&");
                        self.generate_expression(&args[0]);
                        self.emit(")");
                    }
                    "type" => {
                        self.emit("xe_type(&");
                        self.generate_expression(&args[0]);
                        self.emit(")");
                    }
                    "convert" => {
                        self.emit("xe_convert(&");
                        self.generate_expression(&args[0]);
                        self.emit(", &");
                        self.generate_expression(&args[1]);
                        self.emit(")");
                    }
                    _ => {
                        // User-defined function
                        self.emit(&format!("{}(", Self::sanitize_name(name)));
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                self.emit(", ");
                            }
                            self.generate_expression(arg);
                        }
                        self.emit(")");
                    }
                }
            }
            ExpressionKind::Index { object, index } => {
                self.emit("xe_index(&");
                self.generate_expression(object);
                self.emit(", &");
                self.generate_expression(index);
                self.emit(")");
            }
        }
    }

    fn emit(&mut self, s: &str) {
        self.output.push_str(s);
    }

    fn emit_indent(&mut self) {
        for _ in 0..self.indent_level {
            self.output.push_str("    ");
        }
    }

    fn sanitize_name(name: &str) -> String {
        // Prefix with xe_ to avoid Rust keyword conflicts
        match name {
            "type" | "fn" | "let" | "mut" | "if" | "else" | "for" | "while" | "loop" | "match"
            | "return" | "break" | "continue" | "struct" | "enum" | "impl" | "trait" | "pub"
            | "mod" | "use" | "self" | "super" | "crate" | "const" | "static" | "ref" | "move"
            | "async" | "await" | "dyn" | "where" | "in" | "as" | "true" | "false" => {
                format!("xe_{}", name)
            }
            _ => name.to_string(),
        }
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}
