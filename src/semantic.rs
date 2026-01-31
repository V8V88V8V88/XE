use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::error::{XeError, XeErrorKind, XeResult};

const BUILTINS: &[(&str, Option<usize>)] = &[
    ("print", None),      // variadic
    ("input", Some(1)),   // 1 arg (prompt)
    ("length", Some(1)),  // 1 arg
    ("type", Some(1)),    // 1 arg
    ("convert", Some(2)), // 2 args (value, target_type)
];

pub struct SemanticAnalyzer {
    scopes: Vec<HashSet<String>>,
    functions: HashMap<String, usize>, // name -> param count
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        for (name, param_count) in BUILTINS {
            // Use usize::MAX for variadic functions
            functions.insert(name.to_string(), param_count.unwrap_or(usize::MAX));
        }

        Self {
            scopes: vec![HashSet::new()],
            functions,
        }
    }

    pub fn analyze(&mut self, program: &Program) -> XeResult<()> {
        // First pass: collect function definitions
        for stmt in &program.statements {
            if let StatementKind::FunctionDef { name, params, .. } = &stmt.kind {
                if BUILTINS.iter().any(|(n, _)| n == name) {
                    return Err(XeError::new(
                        XeErrorKind::CannotRedefineBuiltin(name.clone()),
                        Some(stmt.span.clone()),
                    ));
                }
                self.functions.insert(name.clone(), params.len());
            }
        }

        // Second pass: analyze statements
        for stmt in &program.statements {
            self.analyze_statement(stmt)?;
        }

        Ok(())
    }

    fn analyze_statement(&mut self, stmt: &Statement) -> XeResult<()> {
        match &stmt.kind {
            StatementKind::Assignment { name, value } => {
                self.analyze_expression(value)?;
                self.define_variable(name);
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                self.analyze_expression(condition)?;
                self.push_scope();
                for s in then_block {
                    self.analyze_statement(s)?;
                }
                self.pop_scope();

                if let Some(else_stmts) = else_block {
                    self.push_scope();
                    for s in else_stmts {
                        self.analyze_statement(s)?;
                    }
                    self.pop_scope();
                }
            }
            StatementKind::Repeat { count, body } => {
                self.analyze_expression(count)?;
                self.push_scope();
                for s in body {
                    self.analyze_statement(s)?;
                }
                self.pop_scope();
            }
            StatementKind::FunctionDef { name: _, params, body } => {
                self.push_scope();
                for param in params {
                    self.define_variable(param);
                }
                for s in body {
                    self.analyze_statement(s)?;
                }
                self.pop_scope();
            }
            StatementKind::Return { value } => {
                if let Some(expr) = value {
                    self.analyze_expression(expr)?;
                }
            }
            StatementKind::Expression(expr) => {
                self.analyze_expression(expr)?;
            }
        }
        Ok(())
    }

    fn analyze_expression(&mut self, expr: &Expression) -> XeResult<()> {
        match &expr.kind {
            ExpressionKind::Number(_)
            | ExpressionKind::String(_)
            | ExpressionKind::Boolean(_) => {}

            ExpressionKind::Identifier(name) => {
                if !self.is_variable_defined(name) {
                    return Err(XeError::new(
                        XeErrorKind::UndefinedVariable(name.clone()),
                        Some(expr.span.clone()),
                    ));
                }
            }

            ExpressionKind::List(elements) => {
                for elem in elements {
                    self.analyze_expression(elem)?;
                }
            }

            ExpressionKind::BinaryOp { left, right, .. } => {
                self.analyze_expression(left)?;
                self.analyze_expression(right)?;
            }

            ExpressionKind::UnaryOp { operand, .. } => {
                self.analyze_expression(operand)?;
            }

            ExpressionKind::FunctionCall { name, args } => {
                // Check if function exists
                if let Some(&expected_params) = self.functions.get(name) {
                    // Check argument count (skip for variadic functions)
                    if expected_params != usize::MAX && args.len() != expected_params {
                        return Err(XeError::new(
                            XeErrorKind::WrongArgumentCount {
                                name: name.clone(),
                                expected: expected_params,
                                got: args.len(),
                            },
                            Some(expr.span.clone()),
                        ));
                    }
                } else {
                    return Err(XeError::new(
                        XeErrorKind::UndefinedFunction(name.clone()),
                        Some(expr.span.clone()),
                    ));
                }

                for arg in args {
                    self.analyze_expression(arg)?;
                }
            }

            ExpressionKind::Index { object, index } => {
                self.analyze_expression(object)?;
                self.analyze_expression(index)?;
            }
        }
        Ok(())
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
        self.scopes.iter().any(|scope| scope.contains(name))
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
