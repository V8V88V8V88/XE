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

    pub fn generate(&mut self, program: &TypedProgram) -> String {
        // Generate runtime and prelude
        self.emit_prelude();

        // Collect user-defined functions
        let mut functions = Vec::new();
        let mut main_statements = Vec::new();

        for stmt in &program.statements {
            if matches!(stmt.kind, TypedStatementKind::FunctionDef { .. }) {
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
            r#"#![allow(dead_code, unused_mut, unused_variables, non_snake_case, unused_parens)]
use std::io::{self, Write};

#[derive(Clone, Debug)]
enum XeValue {
    Number(f64),
    Text(String),
    Boolean(bool),
    List(Vec<XeValue>),
}

impl From<f64> for XeValue {
    fn from(n: f64) -> Self { XeValue::Number(n) }
}

impl From<bool> for XeValue {
    fn from(b: bool) -> Self { XeValue::Boolean(b) }
}

impl From<String> for XeValue {
    fn from(s: String) -> Self { XeValue::Text(s) }
}

impl From<&str> for XeValue {
    fn from(s: &str) -> Self { XeValue::Text(s.to_string()) }
}

impl From<XeValue> for f64 {
    fn from(v: XeValue) -> Self { xe_expect_number(&v, "conversion") }
}

impl From<XeValue> for bool {
    fn from(v: XeValue) -> Self { v.as_bool() }
}

impl From<XeValue> for String {
    fn from(v: XeValue) -> Self { v.to_string() }
}

impl From<XeValue> for Vec<XeValue> {
    fn from(v: XeValue) -> Self { v.as_list() }
}

impl<T: Into<XeValue>> From<Vec<T>> for XeValue {
    fn from(v: Vec<T>) -> Self {
        XeValue::List(v.into_iter().map(|item| item.into()).collect())
    }
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

    fn as_f64(&self) -> f64 {
        xe_expect_number(self, "conversion")
    }

    fn as_string(&self) -> String {
        self.to_string()
    }

    fn as_list(&self) -> Vec<XeValue> {
        match self {
            XeValue::List(l) => l.clone(),
            _ => xe_runtime_error(&format!(
                "expected list, got {}",
                self.type_name()
            )),
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

fn xe_builtin_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn xe_builtin_length(value: &XeValue) -> f64 {
    match value {
        XeValue::Text(s) => s.len() as f64,
        XeValue::List(l) => l.len() as f64,
        _ => xe_runtime_error(&format!(
            "length() expected text or list, got {}",
            value.type_name()
        )),
    }
}

fn xe_builtin_type(value: &XeValue) -> String {
    value.type_name().to_string()
}

fn xe_builtin_convert(value: &XeValue, target_type: &str) -> XeValue {
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

fn xe_add_dynamic(left: XeValue, right: XeValue) -> XeValue {
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

fn xe_sub_native(left: f64, right: f64) -> f64 { left - right }
fn xe_mul_native(left: f64, right: f64) -> f64 { left * right }
fn xe_div_native(left: f64, right: f64) -> f64 {
    if right == 0.0 { xe_runtime_error("division by zero"); }
    left / right
}
fn xe_mod_native(left: f64, right: f64) -> f64 {
    if right == 0.0 { xe_runtime_error("modulo by zero"); }
    left % right
}

fn xe_eq(left: &XeValue, right: &XeValue) -> bool {
    match (left, right) {
        (XeValue::Number(a), XeValue::Number(b)) => {
            if a == b {
                true
            } else {
                (a - b).abs() <= f64::EPSILON * a.abs().max(b.abs()).max(1.0)
            }
        }
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

fn xe_index_check(idx: f64) -> usize {
    if idx < 0.0 || idx.fract() != 0.0 {
        xe_runtime_error(&format!("index access expected a non-negative integer, got {}", idx));
    }
    idx as usize
}

fn xe_vec_index<T: Clone>(list: &[T], idx: f64) -> T {
    let i = xe_index_check(idx);
    if i >= list.len() {
        xe_runtime_error(&format!("list index {} out of bounds", i));
    }
    list[i].clone()
}

fn xe_index(obj: &XeValue, idx: f64) -> XeValue {
    let i = xe_index_check(idx);
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

    fn generate_statement(&mut self, stmt: &TypedStatement) {
        match &stmt.kind {
            TypedStatementKind::Assignment { name, value } => {
                self.emit_indent();
                let sanitized = Self::sanitize_name(name);
                
                if self.scopes.len() == 1 {
                    let rust_type = value.ty.to_rust_type();
                    if !self.is_variable_defined(name) {
                        self.emit(&format!("let mut {}: {} = ", sanitized, rust_type));
                        self.generate_expression(value);
                        self.emit(";\n");
                        self.define_variable(name);
                    } else {
                        self.emit(&format!("{} = ", sanitized));
                        self.generate_expression_with_coercion(value, &value.ty);
                        self.emit(";\n");
                    }
                    
                    self.emit_indent();
                    self.emit(&format!("xe_set_global(\"{}\", XeValue::from({}.clone()));\n", sanitized, sanitized));
                } else {
                    if self.is_variable_defined(name) {
                        self.emit(&format!("{} = ", sanitized));
                        self.generate_expression_with_coercion(value, &value.ty);
                        self.emit(";\n");
                    } else {
                        self.define_variable(name);
                        let rust_type = value.ty.to_rust_type();
                        self.emit(&format!("let mut {}: {} = ", sanitized, rust_type));
                        self.generate_expression(value);
                        self.emit(";\n");
                    }
                }
            }
            TypedStatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                self.emit_indent();
                self.emit("if ");
                self.generate_expression_with_coercion(condition, &XeType::Boolean);
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
            TypedStatementKind::While { condition, body } => {
                self.emit_indent();
                self.emit("while ");
                self.generate_expression_with_coercion(condition, &XeType::Boolean);
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
            TypedStatementKind::Repeat { count, body } => {
                self.emit_indent();
                self.emit("for _ in 0..(");
                self.emit("xe_expect_non_negative_integer(&XeValue::from(");
                self.generate_expression(count);
                self.emit("), \"repeat loop count\")");
                self.emit(" as usize) {\n");
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
            TypedStatementKind::For {
                variable,
                iterable,
                body,
            } => {
                self.emit_indent();
                self.emit("for __xe_loop_value in xe_iter(&XeValue::from(");
                self.generate_expression(iterable);
                self.emit(")) {\n");
                self.indent_level += 1;
                self.push_scope();
                self.define_variable(variable);
                self.emit_indent();
                
                let elem_ty = match &iterable.ty {
                    XeType::List(inner) => (**inner).clone(),
                    XeType::Text => XeType::Text,
                    _ => XeType::Unknown,
                };
                
                let rust_type = elem_ty.to_rust_type();
                self.emit(&format!("let mut {}: {} = ", Self::sanitize_name(variable), rust_type));
                
                match &elem_ty {
                    XeType::Number => self.emit("__xe_loop_value.as_f64();\n"),
                    XeType::Boolean => self.emit("__xe_loop_value.as_bool();\n"),
                    XeType::Text => self.emit("__xe_loop_value.as_string();\n"),
                    XeType::List(inner) => match **inner {
                        XeType::Number => self.emit(
                            "__xe_loop_value.as_list().into_iter().map(|v| v.as_f64()).collect();\n",
                        ),
                        XeType::Boolean => self.emit(
                            "__xe_loop_value.as_list().into_iter().map(|v| v.as_bool()).collect();\n",
                        ),
                        XeType::Text => self.emit(
                            "__xe_loop_value.as_list().into_iter().map(|v| v.as_string()).collect();\n",
                        ),
                        _ => self.emit("__xe_loop_value.as_list();\n"),
                    },
                    _ => self.emit("__xe_loop_value;\n"),
                }

                for s in body {
                    self.generate_statement(s);
                }
                self.pop_scope();
                self.indent_level -= 1;
                self.emit_indent();
                self.emit("}\n");
            }
            TypedStatementKind::FunctionDef { name, params, body, return_type } => {
                self.emit(&format!(
                    "fn {}({}) -> {} {{\n",
                    Self::sanitize_name(name),
                    params
                        .iter()
                        .map(|(p, ty)| {
                            format!("mut {}: {}", Self::sanitize_name(p), ty.to_rust_type())
                        })
                        .collect::<Vec<_>>()
                        .join(", "),
                    return_type.to_rust_type()
                ));
                self.indent_level += 1;
                self.push_scope();
                for (param, _) in params {
                    self.define_variable(param);
                }

                for s in body {
                    self.generate_statement(s);
                }

                let ends_with_return = body
                    .last()
                    .map(|s| matches!(s.kind, TypedStatementKind::Return { .. }))
                    .unwrap_or(false);
                if !ends_with_return {
                    self.emit_indent();
                    match return_type {
                        XeType::Number => self.emit("0.0\n"),
                        XeType::Boolean => self.emit("false\n"),
                        XeType::Text => self.emit("String::new()\n"),
                        XeType::Void => self.emit("()\n"),
                        _ => self.emit("XeValue::from(0.0).into()\n"),
                    }
                }

                self.pop_scope();
                self.indent_level -= 1;
                self.emit("}\n");
            }
            TypedStatementKind::Return { value } => {
                self.emit_indent();
                self.emit("return ");
                if let Some(expr) = value {
                    self.emit("(");
                    self.generate_expression(expr);
                    self.emit(").into()");
                }
                self.emit(";\n");
            }
            TypedStatementKind::Break => {
                self.emit_indent();
                self.emit("break;\n");
            }
            TypedStatementKind::Continue => {
                self.emit_indent();
                self.emit("continue;\n");
            }
            TypedStatementKind::Expression(expr) => {
                self.emit_indent();
                self.generate_expression(expr);
                self.emit(";\n");
            }
        }
    }

    fn generate_expression_with_coercion(&mut self, expr: &TypedExpression, target_ty: &XeType) {
        if expr.ty == *target_ty {
            self.generate_expression(expr);
        } else {
            match target_ty {
                XeType::Number => {
                    self.emit("(XeValue::from(");
                    self.generate_expression(expr);
                    self.emit(").as_f64())");
                }
                XeType::Text => {
                    self.emit("(XeValue::from(");
                    self.generate_expression(expr);
                    self.emit(").as_string())");
                }
                XeType::Boolean => {
                    self.emit("(XeValue::from(");
                    self.generate_expression(expr);
                    self.emit(").as_bool())");
                }
                XeType::Unknown => {
                    self.emit("XeValue::from(");
                    self.generate_expression(expr);
                    self.emit(")");
                }
                _ => self.generate_expression(expr),
            }
        }
    }

    fn generate_expression(&mut self, expr: &TypedExpression) {
        match &expr.kind {
            TypedExpressionKind::Number(n) => {
                self.emit(&format!("{:?}f64", n));
            }
            TypedExpressionKind::String(s) => {
                self.emit(&format!("{:?}.to_string()", s));
            }
            TypedExpressionKind::Boolean(b) => {
                self.emit(&format!("{}", b));
            }
            TypedExpressionKind::List(elements) => {
                if let XeType::List(inner) = &expr.ty {
                    if **inner != XeType::Unknown {
                        self.emit("vec![");
                        for (i, elem) in elements.iter().enumerate() {
                            if i > 0 {
                                self.emit(", ");
                            }
                            self.generate_expression(elem);
                        }
                        self.emit("]");
                        return;
                    }
                }
                
                self.emit("vec![");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        self.emit(", ");
                    }
                    self.emit("XeValue::from(");
                    self.generate_expression(elem);
                    self.emit(")");
                }
                self.emit("]");
            }
            TypedExpressionKind::Identifier(name) => {
                let sanitized = Self::sanitize_name(name);
                if self.is_variable_defined(name) {
                    self.emit(&format!("{}.clone()", sanitized));
                } else {
                    let rust_type = match expr.ty {
                        XeType::Number => "val.as_f64()",
                        XeType::Boolean => "val.as_bool()",
                        XeType::Text => "val.as_string()",
                        _ => "val",
                    };
                    self.emit(&format!("{{ let val = xe_get_global(\"{}\"); {} }}", sanitized, rust_type));
                }
            }
            TypedExpressionKind::BinaryOp { left, op, right } => {
                match op {
                    BinaryOperator::Equal => {
                        self.emit("xe_eq(&XeValue::from(");
                        self.generate_expression(left);
                        self.emit("), &XeValue::from(");
                        self.generate_expression(right);
                        self.emit("))");
                    }
                    BinaryOperator::NotEqual => {
                        self.emit("!xe_eq(&XeValue::from(");
                        self.generate_expression(left);
                        self.emit("), &XeValue::from(");
                        self.generate_expression(right);
                        self.emit("))");
                    }
                    BinaryOperator::Add => {
                        if left.ty == XeType::Number && right.ty == XeType::Number {
                            self.emit("(");
                            self.generate_expression(left);
                            self.emit(" + ");
                            self.generate_expression(right);
                            self.emit(")");
                        } else if left.ty == XeType::Text || right.ty == XeType::Text {
                            self.emit("String::from(xe_add_dynamic(XeValue::from(");
                            self.generate_expression(left);
                            self.emit("), XeValue::from(");
                            self.generate_expression(right);
                            self.emit(")))");
                        } else {
                            self.emit("xe_add_dynamic(XeValue::from(");
                            self.generate_expression(left);
                            self.emit("), XeValue::from(");
                            self.generate_expression(right);
                            self.emit("))");
                            if let XeType::List(inner) = &expr.ty {
                                match **inner {
                                    XeType::Number => self.emit(
                                        ".as_list().into_iter().map(|v| v.as_f64()).collect()",
                                    ),
                                    XeType::Boolean => self.emit(
                                        ".as_list().into_iter().map(|v| v.as_bool()).collect()",
                                    ),
                                    XeType::Text => self.emit(
                                        ".as_list().into_iter().map(|v| v.as_string()).collect()",
                                    ),
                                    _ => self.emit(".as_list()"),
                                }
                            }
                        }
                    }
                    BinaryOperator::Subtract => {
                        self.emit("xe_sub_native(");
                        self.generate_expression_with_coercion(left, &XeType::Number);
                        self.emit(", ");
                        self.generate_expression_with_coercion(right, &XeType::Number);
                        self.emit(")");
                    }
                    BinaryOperator::Multiply => {
                        self.emit("xe_mul_native(");
                        self.generate_expression_with_coercion(left, &XeType::Number);
                        self.emit(", ");
                        self.generate_expression_with_coercion(right, &XeType::Number);
                        self.emit(")");
                    }
                    BinaryOperator::Divide => {
                        self.emit("xe_div_native(");
                        self.generate_expression_with_coercion(left, &XeType::Number);
                        self.emit(", ");
                        self.generate_expression_with_coercion(right, &XeType::Number);
                        self.emit(")");
                    }
                    BinaryOperator::Modulo => {
                        self.emit("xe_mod_native(");
                        self.generate_expression_with_coercion(left, &XeType::Number);
                        self.emit(", ");
                        self.generate_expression_with_coercion(right, &XeType::Number);
                        self.emit(")");
                    }
                    BinaryOperator::Less => {
                        self.emit("(");
                        self.generate_expression_with_coercion(left, &XeType::Number);
                        self.emit(" < ");
                        self.generate_expression_with_coercion(right, &XeType::Number);
                        self.emit(")");
                    }
                    BinaryOperator::Greater => {
                        self.emit("(");
                        self.generate_expression_with_coercion(left, &XeType::Number);
                        self.emit(" > ");
                        self.generate_expression_with_coercion(right, &XeType::Number);
                        self.emit(")");
                    }
                    BinaryOperator::LessEqual => {
                        self.emit("(");
                        self.generate_expression_with_coercion(left, &XeType::Number);
                        self.emit(" <= ");
                        self.generate_expression_with_coercion(right, &XeType::Number);
                        self.emit(")");
                    }
                    BinaryOperator::GreaterEqual => {
                        self.emit("(");
                        self.generate_expression_with_coercion(left, &XeType::Number);
                        self.emit(" >= ");
                        self.generate_expression_with_coercion(right, &XeType::Number);
                        self.emit(")");
                    }
                    BinaryOperator::And => {
                        self.emit("(");
                        self.generate_expression_with_coercion(left, &XeType::Boolean);
                        self.emit(" && ");
                        self.generate_expression_with_coercion(right, &XeType::Boolean);
                        self.emit(")");
                    }
                    BinaryOperator::Or => {
                        self.emit("(");
                        self.generate_expression_with_coercion(left, &XeType::Boolean);
                        self.emit(" || ");
                        self.generate_expression_with_coercion(right, &XeType::Boolean);
                        self.emit(")");
                    }
                }
            }
            TypedExpressionKind::UnaryOp { op, operand } => match op {
                UnaryOperator::Negate => {
                    self.emit("-(");
                    self.generate_expression_with_coercion(operand, &XeType::Number);
                    self.emit(")");
                }
                UnaryOperator::Not => {
                    self.emit("!(");
                    self.generate_expression_with_coercion(operand, &XeType::Boolean);
                    self.emit(")");
                }
            },
            TypedExpressionKind::FunctionCall { name, args } => {
                match name.as_str() {
                    "print" => {
                        self.emit("xe_builtin_print(vec![");
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                self.emit(", ");
                            }
                            self.emit("XeValue::from(");
                            self.generate_expression(arg);
                            self.emit(")");
                        }
                        self.emit("])");
                    }
                    "input" => {
                        self.emit("xe_builtin_input(&");
                        if !args.is_empty() {
                            self.generate_expression_with_coercion(&args[0], &XeType::Text);
                        } else {
                            self.emit("String::new()");
                        }
                        self.emit(")");
                    }
                    "length" => {
                        self.emit("xe_builtin_length(&XeValue::from(");
                        self.generate_expression(&args[0]);
                        self.emit("))");
                    }
                    "type" => {
                        self.emit("xe_builtin_type(&XeValue::from(");
                        self.generate_expression(&args[0]);
                        self.emit("))");
                    }
                    "convert" => {
                        self.emit("xe_builtin_convert(&XeValue::from(");
                        self.generate_expression(&args[0]);
                        self.emit("), &");
                        self.generate_expression_with_coercion(&args[1], &XeType::Text);
                        self.emit(")");
                    }
                    _ => {
                        self.emit(&format!("{}(", Self::sanitize_name(name)));
                        for (i, arg) in args.iter().enumerate() {
                            if i > 0 {
                                self.emit(", ");
                            }
                            self.emit("XeValue::from(");
                            self.generate_expression(arg);
                            self.emit(").into()");
                        }
                        self.emit(")");
                    }
                }
            }
            TypedExpressionKind::Index { object, index } => {
                if let XeType::List(inner) = &object.ty {
                    if **inner != XeType::Unknown {
                        self.emit("xe_vec_index(&(");
                        self.generate_expression(object);
                        self.emit("), ");
                        self.generate_expression_with_coercion(index, &XeType::Number);
                        self.emit(")");
                        return;
                    }
                }

                self.emit("xe_index(&XeValue::from(");
                self.generate_expression(object);
                self.emit("), ");
                self.generate_expression_with_coercion(index, &XeType::Number);
                self.emit(")");
                if expr.ty == XeType::Text {
                    self.emit(".as_string()");
                } else if expr.ty == XeType::Number {
                    self.emit(".as_f64()");
                } else if expr.ty == XeType::Boolean {
                    self.emit(".as_bool()");
                }
            }
            TypedExpressionKind::Wrap(expr) => {
                self.emit("XeValue::from(");
                self.generate_expression(expr);
                self.emit(")");
            }
            TypedExpressionKind::Unwrap(expr, ty) => {
                match ty {
                    XeType::Number => {
                        self.emit("(XeValue::from(");
                        self.generate_expression(expr);
                        self.emit(").as_f64())");
                    }
                    XeType::Boolean => {
                        self.emit("(XeValue::from(");
                        self.generate_expression(expr);
                        self.emit(").as_bool())");
                    }
                    XeType::Text => {
                        self.emit("(XeValue::from(");
                        self.generate_expression(expr);
                        self.emit(").as_string())");
                    }
                    _ => self.generate_expression(expr),
                }
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

    fn sanitize_name(name: &str) -> String {
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
