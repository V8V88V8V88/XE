use std::collections::HashMap;

use crate::ast::*;
use crate::error::{Span, XeError, XeErrorKind, XeResult};

#[derive(Clone)]
struct SymbolInfo {
    ty: XeType,
    defined_at: Span,
}

#[derive(Clone)]
struct FunctionSignature {
    params: Option<Vec<XeType>>, // None means variadic
    return_type: XeType,
}

const BUILTINS: &[(&str, Option<usize>)] = &[
    ("print", None),      // variadic
    ("input", Some(1)),   // 1 arg (prompt)
    ("length", Some(1)),  // 1 arg
    ("type", Some(1)),    // 1 arg
    ("convert", Some(2)), // 2 args (value, target_type)
];

fn get_builtin_signature(name: &str) -> Option<FunctionSignature> {
    match name {
        "print" => Some(FunctionSignature {
            params: None,
            return_type: XeType::Void,
        }),
        "input" => Some(FunctionSignature {
            params: Some(vec![XeType::Text]),
            return_type: XeType::Text,
        }),
        "length" => Some(FunctionSignature {
            params: Some(vec![XeType::Unknown]), // Can be list or text
            return_type: XeType::Number,
        }),
        "type" => Some(FunctionSignature {
            params: Some(vec![XeType::Unknown]),
            return_type: XeType::Text,
        }),
        "convert" => Some(FunctionSignature {
            params: Some(vec![XeType::Unknown, XeType::Text]),
            return_type: XeType::Unknown, // Depends on target_type string literal
        }),
        _ => None,
    }
}

pub struct SemanticAnalyzer {
    scopes: Vec<HashMap<String, SymbolInfo>>,
    functions: HashMap<String, FunctionSignature>,
    loop_depth: usize,
    function_depth: usize,
    current_function_return_type: Option<XeType>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        let mut functions = HashMap::new();
        for (name, _) in BUILTINS {
            if let Some(sig) = get_builtin_signature(name) {
                functions.insert(name.to_string(), sig);
            }
        }

        Self {
            scopes: vec![HashMap::new()],
            functions,
            loop_depth: 0,
            function_depth: 0,
            current_function_return_type: None,
        }
    }

    pub fn analyze(&mut self, program: &Program) -> XeResult<TypedProgram> {
        // Pass 1: Collect function signatures and top-level variables
        for stmt in &program.statements {
            match &stmt.kind {
                StatementKind::FunctionDef {
                    name,
                    params,
                    body: _,
                } => {
                    if BUILTINS.iter().any(|(n, _)| n == name) {
                        return Err(XeError::new(
                            XeErrorKind::CannotRedefineBuiltin(name.clone()),
                            Some(stmt.span.clone()),
                        ));
                    }
                    if self.functions.contains_key(name) {
                        return Err(XeError::new(
                            XeErrorKind::DuplicateFunction(name.clone()),
                            Some(stmt.span.clone()),
                        ));
                    }
                    
                    self.functions.insert(name.clone(), FunctionSignature {
                        params: Some(vec![XeType::Unknown; params.len()]),
                        return_type: XeType::Unknown,
                    });
                }
                StatementKind::Assignment { name, .. }
                    if !self.is_variable_defined_in_current_scope(name) =>
                {
                    self.define_variable(name, XeType::Unknown, &stmt.span)?;
                }
                _ => {}
            }
        }

        // Pass 2: Full semantic analysis and IR production
        let mut typed_statements = Vec::new();
        for stmt in &program.statements {
            typed_statements.push(self.analyze_statement(stmt)?);
        }

        Ok(TypedProgram { statements: typed_statements })
    }

    fn analyze_statement(&mut self, stmt: &Statement) -> XeResult<TypedStatement> {
        let kind = match &stmt.kind {
            StatementKind::Import { .. } | StatementKind::FromImport { .. } => {
                // Imports are handled during linking
                TypedStatementKind::Expression(TypedExpression {
                    kind: TypedExpressionKind::Boolean(true),
                    ty: XeType::Boolean,
                    span: stmt.span.clone(),
                })
            }
            StatementKind::Assignment { name, value } => {
                let mut typed_value = self.analyze_expression(value)?;
                let ty = typed_value.ty.clone();
                
                if let Some(existing) = self.get_symbol_info(name) {
                    if existing.ty != XeType::Unknown && !existing.ty.is_compatible(&ty) {
                        return Err(XeError::new(
                            XeErrorKind::TypeMismatch {
                                expected: format!("{} (defined at line {})", existing.ty, existing.defined_at.line),
                                got: ty.name(),
                            },
                            Some(stmt.span.clone()),
                        ));
                    }
                    if existing.ty == XeType::Unknown {
                        self.update_variable_type(name, ty);
                    } else if existing.ty != XeType::Unknown && ty == XeType::Unknown {
                        // Coerce dynamic value to existing native variable
                        typed_value = self.unwrap_to(typed_value, existing.ty.clone());
                    }
                } else {
                    self.define_variable(name, ty, &stmt.span)?;
                }
                
                TypedStatementKind::Assignment {
                    name: name.clone(),
                    value: typed_value,
                }
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                let mut typed_cond = self.analyze_expression(condition)?;
                if !typed_cond.ty.is_compatible(&XeType::Boolean) {
                    return Err(XeError::new(
                        XeErrorKind::TypeMismatch {
                            expected: "boolean".to_string(),
                            got: typed_cond.ty.name(),
                        },
                        Some(condition.span.clone()),
                    ));
                }
                if typed_cond.ty == XeType::Unknown {
                    typed_cond = self.unwrap_to(typed_cond, XeType::Boolean);
                }

                self.push_scope();
                let mut typed_then = Vec::new();
                for s in then_block {
                    typed_then.push(self.analyze_statement(s)?);
                }
                self.pop_scope();

                let mut typed_else = None;
                if let Some(else_stmts) = else_block {
                    self.push_scope();
                    let mut else_block_typed = Vec::new();
                    for s in else_stmts {
                        else_block_typed.push(self.analyze_statement(s)?);
                    }
                    typed_else = Some(else_block_typed);
                    self.pop_scope();
                }

                TypedStatementKind::If {
                    condition: typed_cond,
                    then_block: typed_then,
                    else_block: typed_else,
                }
            }
            StatementKind::While { condition, body } => {
                let mut typed_cond = self.analyze_expression(condition)?;
                if !typed_cond.ty.is_compatible(&XeType::Boolean) {
                    return Err(XeError::new(
                        XeErrorKind::TypeMismatch {
                            expected: "boolean".to_string(),
                            got: typed_cond.ty.name(),
                        },
                        Some(condition.span.clone()),
                    ));
                }
                if typed_cond.ty == XeType::Unknown {
                    typed_cond = self.unwrap_to(typed_cond, XeType::Boolean);
                }

                self.loop_depth += 1;
                self.push_scope();
                let mut typed_body = Vec::new();
                for s in body {
                    typed_body.push(self.analyze_statement(s)?);
                }
                self.pop_scope();
                self.loop_depth -= 1;

                TypedStatementKind::While {
                    condition: typed_cond,
                    body: typed_body,
                }
            }
            StatementKind::Repeat { count, body } => {
                let mut typed_count = self.analyze_expression(count)?;
                if !typed_count.ty.is_compatible(&XeType::Number) {
                    return Err(XeError::new(
                        XeErrorKind::TypeMismatch {
                            expected: "number".to_string(),
                            got: typed_count.ty.name(),
                        },
                        Some(count.span.clone()),
                    ));
                }
                if typed_count.ty == XeType::Unknown {
                    typed_count = self.unwrap_to(typed_count, XeType::Number);
                }

                self.loop_depth += 1;
                self.push_scope();
                let mut typed_body = Vec::new();
                for s in body {
                    typed_body.push(self.analyze_statement(s)?);
                }
                self.pop_scope();
                self.loop_depth -= 1;

                TypedStatementKind::Repeat {
                    count: typed_count,
                    body: typed_body,
                }
            }
            StatementKind::For {
                variable,
                iterable,
                body,
            } => {
                let typed_iter = self.analyze_expression(iterable)?;
                let elem_ty = match &typed_iter.ty {
                    XeType::List(inner) => *inner.clone(),
                    XeType::Text => XeType::Text,
                    XeType::Unknown => XeType::Unknown,
                    _ => {
                        return Err(XeError::new(
                            XeErrorKind::TypeMismatch {
                                expected: "list or text".to_string(),
                                got: typed_iter.ty.name(),
                            },
                            Some(iterable.span.clone()),
                        ));
                    }
                };

                self.loop_depth += 1;
                self.push_scope();
                self.define_variable(variable, elem_ty, &stmt.span)?;
                let mut typed_body = Vec::new();
                for s in body {
                    typed_body.push(self.analyze_statement(s)?);
                }
                self.pop_scope();
                self.loop_depth -= 1;

                TypedStatementKind::For {
                    variable: variable.clone(),
                    iterable: typed_iter,
                    body: typed_body,
                }
            }
            StatementKind::FunctionDef {
                name,
                params,
                body,
            } => {
                self.function_depth += 1;
                let old_return_type = self.current_function_return_type.take();
                self.current_function_return_type = Some(XeType::Unknown);

                let global_scope = self.scopes[0].clone();
                let saved_scopes = std::mem::replace(&mut self.scopes, vec![global_scope, HashMap::new()]);
                
                let mut typed_params = Vec::new();
                for param in params {
                    self.define_variable(param, XeType::Unknown, &stmt.span)?;
                    typed_params.push((param.clone(), XeType::Unknown));
                }

                let mut typed_body = Vec::new();
                for s in body {
                    typed_body.push(self.analyze_statement(s)?);
                }

                let return_type = self.current_function_return_type.take().unwrap_or(XeType::Void);
                
                if let Some(sig) = self.functions.get_mut(name) {
                    sig.return_type = return_type.clone();
                }

                self.scopes = saved_scopes;
                self.current_function_return_type = old_return_type;
                self.function_depth -= 1;

                TypedStatementKind::FunctionDef {
                    name: name.clone(),
                    params: typed_params,
                    body: typed_body,
                    return_type,
                }
            }
            StatementKind::Return { value } => {
                if self.function_depth == 0 {
                    return Err(XeError::new(
                        XeErrorKind::ReturnOutsideFunction,
                        Some(stmt.span.clone()),
                    ));
                }
                let typed_value = if let Some(expr) = value {
                    let v = self.analyze_expression(expr)?;
                    if let Some(current_ret) = &mut self.current_function_return_type {
                        if *current_ret == XeType::Unknown {
                            *current_ret = v.ty.clone();
                        } else if *current_ret != v.ty && v.ty != XeType::Unknown {
                            // If we have a mixed return, we must unify to Unknown (boxed)
                            *current_ret = XeType::Unknown;
                        }
                    }
                    Some(v)
                } else {
                    if let Some(current_ret) = &mut self.current_function_return_type {
                        if *current_ret == XeType::Unknown {
                            *current_ret = XeType::Void;
                        }
                    }
                    None
                };

                TypedStatementKind::Return { value: typed_value }
            }
            StatementKind::Break => {
                if self.loop_depth == 0 {
                    return Err(XeError::new(
                        XeErrorKind::BreakOutsideLoop,
                        Some(stmt.span.clone()),
                    ));
                }
                TypedStatementKind::Break
            }
            StatementKind::Continue => {
                if self.loop_depth == 0 {
                    return Err(XeError::new(
                        XeErrorKind::ContinueOutsideLoop,
                        Some(stmt.span.clone()),
                    ));
                }
                TypedStatementKind::Continue
            }
            StatementKind::Expression(expr) => {
                TypedStatementKind::Expression(self.analyze_expression(expr)?)
            }
        };

        Ok(TypedStatement {
            kind,
            span: stmt.span.clone(),
        })
    }

    fn analyze_expression(&mut self, expr: &Expression) -> XeResult<TypedExpression> {
        let span = expr.span.clone();
        let (kind, ty) = match &expr.kind {
            ExpressionKind::Number(n) => (TypedExpressionKind::Number(*n), XeType::Number),
            ExpressionKind::String(s) => (TypedExpressionKind::String(s.clone()), XeType::Text),
            ExpressionKind::Boolean(b) => (TypedExpressionKind::Boolean(*b), XeType::Boolean),

            ExpressionKind::Identifier(name) => {
                if let Some(info) = self.get_symbol_info(name) {
                    (TypedExpressionKind::Identifier(name.clone()), info.ty.clone())
                } else {
                    return Err(XeError::new(
                        XeErrorKind::UndefinedVariable(name.clone()),
                        Some(expr.span.clone()),
                    ));
                }
            }

            ExpressionKind::List(elements) => {
                let mut typed_elements = Vec::new();
                let mut elem_ty: Option<XeType> = None;
                let mut is_mixed = false;

                for elem in elements {
                    let typed_elem = self.analyze_expression(elem)?;
                    match &elem_ty {
                        None => {
                            elem_ty = Some(typed_elem.ty.clone());
                        }
                        Some(prev_ty) => {
                            if prev_ty != &typed_elem.ty || typed_elem.ty == XeType::Unknown {
                                is_mixed = true;
                            }
                        }
                    }
                    typed_elements.push(typed_elem);
                }

                let final_elem_ty = match elem_ty {
                    Some(ty) if !is_mixed => ty,
                    _ => XeType::Unknown,
                };

                (
                    TypedExpressionKind::List(typed_elements),
                    XeType::List(Box::new(final_elem_ty)),
                )
            }

            ExpressionKind::BinaryOp { left, op, right } => {
                let mut l = self.analyze_expression(left)?;
                let mut r = self.analyze_expression(right)?;

                match op {
                    BinaryOperator::Add => {
                        if l.ty == XeType::Number && r.ty == XeType::Number {
                            (TypedExpressionKind::BinaryOp { left: Box::new(l), op: *op, right: Box::new(r) }, XeType::Number)
                        } else if l.ty == XeType::Text || r.ty == XeType::Text {
                            if l.ty != XeType::Text { l = self.wrap_to_unknown(l); }
                            if r.ty != XeType::Text { r = self.wrap_to_unknown(r); }
                            (TypedExpressionKind::BinaryOp { left: Box::new(l), op: *op, right: Box::new(r) }, XeType::Text)
                        } else if let (XeType::List(lt), XeType::List(rt)) = (&l.ty, &r.ty) {
                            let res_ty = if lt.is_compatible(rt) { lt.clone() } else { Box::new(XeType::Unknown) };
                            (TypedExpressionKind::BinaryOp { left: Box::new(l), op: *op, right: Box::new(r) }, XeType::List(res_ty))
                        } else if l.ty == XeType::Unknown || r.ty == XeType::Unknown {
                            (TypedExpressionKind::BinaryOp { left: Box::new(l), op: *op, right: Box::new(r) }, XeType::Unknown)
                        } else {
                            return Err(XeError::new(
                                XeErrorKind::TypeMismatch {
                                    expected: "number, text, or list".to_string(),
                                    got: format!("{} and {}", l.ty, r.ty),
                                },
                                Some(expr.span.clone()),
                            ));
                        }
                    }
                    BinaryOperator::Subtract | BinaryOperator::Multiply | BinaryOperator::Divide | BinaryOperator::Modulo => {
                        if l.ty.is_compatible(&XeType::Number) && r.ty.is_compatible(&XeType::Number) {
                            if l.ty == XeType::Unknown { l = self.unwrap_to(l, XeType::Number); }
                            if r.ty == XeType::Unknown { r = self.unwrap_to(r, XeType::Number); }
                            (TypedExpressionKind::BinaryOp { left: Box::new(l), op: *op, right: Box::new(r) }, XeType::Number)
                        } else {
                            return Err(XeError::new(
                                XeErrorKind::TypeMismatch {
                                    expected: "number".to_string(),
                                    got: format!("{} and {}", l.ty, r.ty),
                                },
                                Some(expr.span.clone()),
                            ));
                        }
                    }
                    BinaryOperator::Equal | BinaryOperator::NotEqual => {
                        (TypedExpressionKind::BinaryOp { left: Box::new(l), op: *op, right: Box::new(r) }, XeType::Boolean)
                    }
                    BinaryOperator::Less | BinaryOperator::Greater | BinaryOperator::LessEqual | BinaryOperator::GreaterEqual => {
                        if l.ty.is_compatible(&XeType::Number) && r.ty.is_compatible(&XeType::Number) {
                            if l.ty == XeType::Unknown { l = self.unwrap_to(l, XeType::Number); }
                            if r.ty == XeType::Unknown { r = self.unwrap_to(r, XeType::Number); }
                            (TypedExpressionKind::BinaryOp { left: Box::new(l), op: *op, right: Box::new(r) }, XeType::Boolean)
                        } else {
                            return Err(XeError::new(
                                XeErrorKind::TypeMismatch {
                                    expected: "number".to_string(),
                                    got: format!("{} and {}", l.ty, r.ty),
                                },
                                Some(expr.span.clone()),
                            ));
                        }
                    }
                    BinaryOperator::And | BinaryOperator::Or => {
                        if l.ty.is_compatible(&XeType::Boolean) && r.ty.is_compatible(&XeType::Boolean) {
                            if l.ty == XeType::Unknown { l = self.unwrap_to(l, XeType::Boolean); }
                            if r.ty == XeType::Unknown { r = self.unwrap_to(r, XeType::Boolean); }
                            (TypedExpressionKind::BinaryOp { left: Box::new(l), op: *op, right: Box::new(r) }, XeType::Boolean)
                        } else {
                            return Err(XeError::new(
                                XeErrorKind::TypeMismatch {
                                    expected: "boolean".to_string(),
                                    got: format!("{} and {}", l.ty, r.ty),
                                },
                                Some(expr.span.clone()),
                            ));
                        }
                    }
                }
            }

            ExpressionKind::UnaryOp { op, operand } => {
                let mut o = self.analyze_expression(operand)?;
                match op {
                    UnaryOperator::Negate => {
                        if o.ty.is_compatible(&XeType::Number) {
                            if o.ty == XeType::Unknown { o = self.unwrap_to(o, XeType::Number); }
                            (TypedExpressionKind::UnaryOp { op: *op, operand: Box::new(o) }, XeType::Number)
                        } else {
                            return Err(XeError::new(
                                XeErrorKind::TypeMismatch {
                                    expected: "number".to_string(),
                                    got: o.ty.name(),
                                },
                                Some(expr.span.clone()),
                            ));
                        }
                    }
                    UnaryOperator::Not => {
                        if o.ty.is_compatible(&XeType::Boolean) {
                            if o.ty == XeType::Unknown { o = self.unwrap_to(o, XeType::Boolean); }
                            (TypedExpressionKind::UnaryOp { op: *op, operand: Box::new(o) }, XeType::Boolean)
                        } else {
                            return Err(XeError::new(
                                XeErrorKind::TypeMismatch {
                                    expected: "boolean".to_string(),
                                    got: o.ty.name(),
                                },
                                Some(expr.span.clone()),
                            ));
                        }
                    }
                }
            }

            ExpressionKind::FunctionCall { name, args } => {
                let sig = if let Some(sig) = self.functions.get(name) {
                    sig.clone()
                } else {
                    return Err(XeError::new(
                        XeErrorKind::UndefinedFunction(name.clone()),
                        Some(expr.span.clone()),
                    ));
                };

                let mut typed_args = Vec::new();
                if let Some(expected_params) = &sig.params {
                    if args.len() != expected_params.len() {
                        return Err(XeError::new(
                            XeErrorKind::WrongArgumentCount {
                                name: name.clone(),
                                expected: expected_params.len(),
                                got: args.len(),
                            },
                            Some(expr.span.clone()),
                        ));
                    }
                    
                    for (i, arg) in args.iter().enumerate() {
                        let mut arg_typed = self.analyze_expression(arg)?;
                        let expected_ty = &expected_params[i];
                        
                        if !arg_typed.ty.is_compatible(expected_ty) {
                             return Err(XeError::new(
                                XeErrorKind::TypeMismatch {
                                    expected: expected_ty.name(),
                                    got: arg_typed.ty.name(),
                                },
                                Some(arg.span.clone()),
                            ));
                        }

                        if *expected_ty == XeType::Unknown && arg_typed.ty != XeType::Unknown {
                            arg_typed = self.wrap_to_unknown(arg_typed);
                        } else if *expected_ty != XeType::Unknown && arg_typed.ty == XeType::Unknown {
                            arg_typed = self.unwrap_to(arg_typed, expected_ty.clone());
                        }

                        typed_args.push(arg_typed);
                    }
                } else {
                    for arg in args {
                        let mut arg_typed = self.analyze_expression(arg)?;
                        if arg_typed.ty != XeType::Unknown {
                            arg_typed = self.wrap_to_unknown(arg_typed);
                        }
                        typed_args.push(arg_typed);
                    }
                }

                (TypedExpressionKind::FunctionCall { name: name.clone(), args: typed_args }, sig.return_type)
            }

            ExpressionKind::Index { object, index } => {
                let obj_typed = self.analyze_expression(object)?;
                let mut idx_typed = self.analyze_expression(index)?;
                
                if !idx_typed.ty.is_compatible(&XeType::Number) {
                    return Err(XeError::new(
                        XeErrorKind::TypeMismatch {
                            expected: "number".to_string(),
                            got: idx_typed.ty.name(),
                        },
                        Some(index.span.clone()),
                    ));
                }
                if idx_typed.ty == XeType::Unknown {
                    idx_typed = self.unwrap_to(idx_typed, XeType::Number);
                }

                let ret_ty = match &obj_typed.ty {
                    XeType::List(inner) => *inner.clone(),
                    XeType::Text => XeType::Text,
                    XeType::Unknown => XeType::Unknown,
                    _ => return Err(XeError::new(
                        XeErrorKind::TypeMismatch {
                            expected: "list or text".to_string(),
                            got: obj_typed.ty.name(),
                        },
                        Some(object.span.clone()),
                    )),
                };

                (TypedExpressionKind::Index { object: Box::new(obj_typed), index: Box::new(idx_typed) }, ret_ty)
            }
        };

        Ok(TypedExpression { kind, ty, span })
    }

    fn wrap_to_unknown(&self, expr: TypedExpression) -> TypedExpression {
        TypedExpression {
            ty: XeType::Unknown,
            span: expr.span.clone(),
            kind: TypedExpressionKind::Wrap(Box::new(expr)),
        }
    }

    fn unwrap_to(&self, expr: TypedExpression, ty: XeType) -> TypedExpression {
        TypedExpression {
            ty: ty.clone(),
            span: expr.span.clone(),
            kind: TypedExpressionKind::Unwrap(Box::new(expr), ty),
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn define_variable(&mut self, name: &str, ty: XeType, span: &Span) -> XeResult<()> {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), SymbolInfo {
                ty,
                defined_at: span.clone(),
            });
        }
        Ok(())
    }

    fn update_variable_type(&mut self, name: &str, ty: XeType) {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(info) = scope.get_mut(name) {
                info.ty = ty;
                return;
            }
        }
    }

    fn get_symbol_info(&self, name: &str) -> Option<&SymbolInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info);
            }
        }
        None
    }

    fn is_variable_defined_in_current_scope(&self, name: &str) -> bool {
        self.scopes.last().map(|s| s.contains_key(name)).unwrap_or(false)
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
