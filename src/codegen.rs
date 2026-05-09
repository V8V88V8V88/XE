use std::collections::HashSet;

use crate::ast::*;

pub struct CodeGenerator {
    output: String,
    indent_level: usize,
    scopes: Vec<HashSet<String>>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            scopes: vec![HashSet::new()],
        }
    }

    pub fn generate(&mut self, program: &Program) -> String {
        // First pass: collect all global variables
        for stmt in &program.statements {
            match &stmt.kind {
                StatementKind::Assignment { name, .. } => {
                    self.define_variable(name);
                }
                StatementKind::FunctionDef { name, body, .. } if name.starts_with("xe_m") => {
                    // Also scan for global assignments inside module init/functions
                    for s in body {
                        if let StatementKind::Assignment { name, .. } = &s.kind {
                            if name.starts_with("xe_m") {
                                self.define_variable(name);
                            }
                        }
                    }
                }
                _ => {}
            }
        }

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

fn xe_runtime_error(message: &str) -> ! {
    eprintln!("Runtime error: {}", message);
    std::process::exit(1);
}

fn xe_expect_number(value: &XeValue, context: &str) -> f64 {
    match value {
        XeValue::Number(n) => *n,
        _ => xe_runtime_error(&format!(
            "{} expected a number, got {}",
            context,
            value.type_name()
        )),
    }
}

fn xe_expect_non_negative_integer(value: &XeValue, context: &str) -> usize {
    let number = xe_expect_number(value, context);
    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
        xe_runtime_error(&format!(
            "{} expected a non-negative integer, got {}",
            context,
            number
        ));
    }
    number as usize
}

fn xe_builtin_print(args: Vec<XeValue>) {
    let output: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    println!("{}", output.join(" "));
}

fn xe_builtin_input(prompt: &XeValue) -> XeValue {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    XeValue::Text(input.trim().to_string())
}

fn xe_builtin_length(value: &XeValue) -> XeValue {
    match value {
        XeValue::Text(s) => XeValue::Number(s.len() as f64),
        XeValue::List(l) => XeValue::Number(l.len() as f64),
        _ => xe_runtime_error(&format!(
            "length() expected text or list, got {}",
            value.type_name()
        )),
    }
}

fn xe_builtin_type(value: &XeValue) -> XeValue {
    XeValue::Text(value.type_name().to_string())
}

fn xe_builtin_convert(value: &XeValue, target: &XeValue) -> XeValue {
    let target_type = match target {
        XeValue::Text(s) => s.as_str(),
        _ => xe_runtime_error(&format!(
            "convert() target must be text, got {}",
            target.type_name()
        )),
    };
    match target_type {
        "number" => match value {
            XeValue::Number(n) => XeValue::Number(*n),
            XeValue::Text(s) => match s.parse::<f64>() {
                Ok(n) => XeValue::Number(n),
                Err(_) => xe_runtime_error(&format!(
                    "cannot convert text '{}' to number",
                    s
                )),
            },
            XeValue::Boolean(b) => XeValue::Number(if *b { 1.0 } else { 0.0 }),
            XeValue::List(_) => xe_runtime_error("cannot convert list to number"),
        },
        "text" => XeValue::Text(value.to_string()),
        "boolean" => XeValue::Boolean(value.as_bool()),
        _ => xe_runtime_error(&format!(
            "unsupported convert() target '{}'",
            target_type
        )),
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
        (XeValue::Number(a), XeValue::Number(b)) => XeValue::Number(a + b),
        _ => xe_runtime_error(&format!(
            "operator '+' is not defined for {} and {}",
            left.type_name(),
            right.type_name()
        )),
    }
}

fn xe_sub(left: XeValue, right: XeValue) -> XeValue {
    XeValue::Number(
        xe_expect_number(&left, "operator '-'") - xe_expect_number(&right, "operator '-'"),
    )
}

fn xe_mul(left: XeValue, right: XeValue) -> XeValue {
    XeValue::Number(
        xe_expect_number(&left, "operator '*'") * xe_expect_number(&right, "operator '*'"),
    )
}

fn xe_div(left: XeValue, right: XeValue) -> XeValue {
    let lhs = xe_expect_number(&left, "operator '/'");
    let rhs = xe_expect_number(&right, "operator '/'");
    if rhs == 0.0 {
        xe_runtime_error("division by zero");
    }
    XeValue::Number(lhs / rhs)
}

fn xe_mod(left: XeValue, right: XeValue) -> XeValue {
    let lhs = xe_expect_number(&left, "operator '%'");
    let rhs = xe_expect_number(&right, "operator '%'");
    if rhs == 0.0 {
        xe_runtime_error("modulo by zero");
    }
    XeValue::Number(lhs % rhs)
}

fn xe_eq(left: &XeValue, right: &XeValue) -> bool {
    match (left, right) {
        (XeValue::Number(a), XeValue::Number(b)) => (a - b).abs() < f64::EPSILON,
        (XeValue::Text(a), XeValue::Text(b)) => a == b,
        (XeValue::Boolean(a), XeValue::Boolean(b)) => a == b,
        (XeValue::List(a), XeValue::List(b)) => {
            if a.len() != b.len() {
                return false;
            }
            for (i, item) in a.iter().enumerate() {
                if !xe_eq(item, &b[i]) {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

fn xe_lt(left: &XeValue, right: &XeValue) -> bool {
    xe_expect_number(left, "operator '<'") < xe_expect_number(right, "operator '<'")
}

fn xe_gt(left: &XeValue, right: &XeValue) -> bool {
    xe_expect_number(left, "operator '>'") > xe_expect_number(right, "operator '>'")
}

fn xe_le(left: &XeValue, right: &XeValue) -> bool {
    xe_expect_number(left, "operator '<='") <= xe_expect_number(right, "operator '<='")
}

fn xe_ge(left: &XeValue, right: &XeValue) -> bool {
    xe_expect_number(left, "operator '>='") >= xe_expect_number(right, "operator '>='")
}

fn xe_index(obj: &XeValue, idx: &XeValue) -> XeValue {
    let i = xe_expect_non_negative_integer(idx, "index access");
    match obj {
        XeValue::List(l) => l
            .get(i)
            .cloned()
            .unwrap_or_else(|| xe_runtime_error(&format!("list index {} out of bounds", i))),
        XeValue::Text(s) => {
            s.chars().nth(i).map(|c| XeValue::Text(c.to_string())).unwrap_or_else(|| {
                xe_runtime_error(&format!("text index {} out of bounds", i))
            })
        }
        _ => xe_runtime_error(&format!(
            "index access expected text or list, got {}",
            obj.type_name()
        )),
    }
}

fn xe_iter(value: &XeValue) -> Vec<XeValue> {
    match value {
        XeValue::List(items) => items.clone(),
        XeValue::Text(s) => s.chars().map(|c| XeValue::Text(c.to_string())).collect(),
        _ => xe_runtime_error(&format!(
            "for-loop iteration expected text or list, got {}",
            value.type_name()
        )),
    }
}

thread_local! {
    static XE_GLOBALS: std::cell::RefCell<std::collections::HashMap<String, XeValue>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

fn xe_get_global(name: &str) -> XeValue {
    XE_GLOBALS.with(|g| g.borrow().get(name).cloned().unwrap_or(XeValue::Number(0.0)))
}

fn xe_set_global(name: &str, value: XeValue) -> XeValue {
    XE_GLOBALS.with(|g| g.borrow_mut().insert(name.to_string(), value.clone()));
    value
}

"#,
        );
    }

    fn generate_statement(&mut self, stmt: &Statement) {
        match &stmt.kind {
            StatementKind::Import { .. } | StatementKind::FromImport { .. } => {}
            StatementKind::Assignment { name, value } => {
                self.emit_indent();
                let sanitized = Self::sanitize_name(name);
                
                // If we are at the top level (scope 0), it's a global assignment.
                // Or if it's already defined as a global.
                if self.scopes.len() == 1 || (self.is_variable_defined(name) && self.get_variable_scope_index(name) == 0) {
                    self.emit(&format!("xe_set_global(\"{}\", ", sanitized));
                    self.generate_expression(value);
                    self.emit(");\n");
                    if self.scopes.len() == 1 {
                        self.define_variable(name);
                    }
                } else if self.is_variable_defined(name) {
                    self.emit(&format!("{} = ", sanitized));
                    self.generate_expression(value);
                    self.emit(";\n");
                } else {
                    self.define_variable(name);
                    self.emit(&format!("let mut {} = ", sanitized));
                    self.generate_expression(value);
                    self.emit(";\n");
                }
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
                self.push_scope();
                for s in then_block {
                    self.generate_statement(s);
                }
                self.pop_scope();
                self.indent_level -= 1;
                self.emit_indent();
                self.emit("}");
                if let Some(else_stmts) = else_block {
                    self.emit(" else {\n");
                    self.indent_level += 1;
                    self.push_scope();
                    for s in else_stmts {
                        self.generate_statement(s);
                    }
                    self.pop_scope();
                    self.indent_level -= 1;
                    self.emit_indent();
                    self.emit("}");
                }
                self.emit("\n");
            }
            StatementKind::While { condition, body } => {
                self.emit_indent();
                self.emit("while ");
                self.generate_condition(condition);
                self.emit(" {\n");
                self.indent_level += 1;
                self.push_scope();
                for s in body {
                    self.generate_statement(s);
                }
                self.pop_scope();
                self.indent_level -= 1;
                self.emit_indent();
                self.emit("}\n");
            }
            StatementKind::Repeat { count, body } => {
                self.emit_indent();
                self.emit("for _ in 0..xe_expect_non_negative_integer(&");
                self.generate_expression(count);
                self.emit(", \"repeat loop count\") {\n");
                self.indent_level += 1;
                self.push_scope();
                for s in body {
                    self.generate_statement(s);
                }
                self.pop_scope();
                self.indent_level -= 1;
                self.emit_indent();
                self.emit("}\n");
            }
            StatementKind::For {
                variable,
                iterable,
                body,
            } => {
                self.emit_indent();
                self.emit("for __xe_loop_value in xe_iter(&");
                self.generate_expression(iterable);
                self.emit(") {\n");
                self.indent_level += 1;
                self.push_scope();
                self.define_variable(variable);
                self.emit_indent();
                self.emit(&format!(
                    "let mut {} = __xe_loop_value;\n",
                    Self::sanitize_name(variable)
                ));
                for s in body {
                    self.generate_statement(s);
                }
                self.pop_scope();
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
                        .map(|p| format!("mut {}: XeValue", Self::sanitize_name(p)))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                self.indent_level += 1;
                self.push_scope();
                for param in params {
                    self.define_variable(param);
                }

                for s in body {
                    self.generate_statement(s);
                }

                let ends_with_return = body
                    .last()
                    .map(|s| matches!(s.kind, StatementKind::Return { .. }))
                    .unwrap_or(false);
                if !ends_with_return {
                    self.emit_indent();
                    self.emit("XeValue::Number(0.0)\n");
                }

                self.pop_scope();
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
            StatementKind::Break => {
                self.emit_indent();
                self.emit("break;\n");
            }
            StatementKind::Continue => {
                self.emit_indent();
                self.emit("continue;\n");
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
                let sanitized = Self::sanitize_name(name);
                if self.is_variable_defined(name) && self.get_variable_scope_index(name) > 0 {
                    self.emit(&format!("{}.clone()", sanitized));
                } else {
                    // Assume global if not local
                    self.emit(&format!("xe_get_global(\"{}\")", sanitized));
                }
            }
            ExpressionKind::BinaryOp { left, op, right } => match op {
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
            },
            ExpressionKind::UnaryOp { op, operand } => match op {
                UnaryOperator::Negate => {
                    self.emit("XeValue::Number(-xe_expect_number(&");
                    self.generate_expression(operand);
                    self.emit(", \"unary '-'\"))");
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
                        self.emit("{ xe_builtin_print(vec![");
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                self.emit(", ");
                            }
                            self.generate_expression(arg);
                        }
                        self.emit("]); XeValue::Number(0.0) }");
                    }
                    "input" => {
                        self.emit("xe_builtin_input(&");
                        if !args.is_empty() {
                            self.generate_expression(&args[0]);
                        } else {
                            self.emit("XeValue::Text(String::new())");
                        }
                        self.emit(")");
                    }
                    "length" => {
                        self.emit("xe_builtin_length(&");
                        self.generate_expression(&args[0]);
                        self.emit(")");
                    }
                    "type" => {
                        self.emit("xe_builtin_type(&");
                        self.generate_expression(&args[0]);
                        self.emit(")");
                    }
                    "convert" => {
                        self.emit("xe_builtin_convert(&");
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

    fn push_scope(&mut self) {
        self.scopes.push(HashSet::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_variable(&mut self, name: &str) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string());
        }
    }

    fn is_variable_defined(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|scope| scope.contains(name))
    }

    fn get_variable_scope_index(&self, name: &str) -> usize {
        for (i, scope) in self.scopes.iter().enumerate().rev() {
            if scope.contains(name) {
                return i;
            }
        }
        0
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
