use std::collections::HashMap;

use crate::ast::*;
use crate::error::{Span, XeError, XeErrorKind, XeResult};

#[derive(Clone)]
struct SymbolInfo {
    ty: XeType,
    defined_at: Span,
    /// False for a module-level variable that is known to exist but whose first
    /// assignment (which fixes its type) has not been analyzed yet.
    declared: bool,
}

/// Return statements seen while analyzing the current function body.
#[derive(Default)]
struct ReturnInfo {
    value_types: Vec<XeType>,
    has_bare_return: bool,
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
    ("append", Some(2)),  // 2 args (list, item)
    ("pop", Some(1)),     // 1 arg (list)
    ("keys", Some(1)),    // 1 arg (map or struct)
    ("values", Some(1)),  // 1 arg (map or struct)
    ("has_key", Some(2)), // 2 args (map/struct, key)
    ("contains", Some(2)),// 2 args (collection, item)
    ("split", Some(2)),   // 2 args (text, delimiter)
    ("join", Some(2)),    // 2 args (list, delimiter)
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
            params: Some(vec![XeType::Unknown]),
            return_type: XeType::Number,
        }),
        "type" => Some(FunctionSignature {
            params: Some(vec![XeType::Unknown]),
            return_type: XeType::Text,
        }),
        "convert" => Some(FunctionSignature {
            params: Some(vec![XeType::Unknown, XeType::Text]),
            return_type: XeType::Unknown,
        }),
        "append" => Some(FunctionSignature {
            params: Some(vec![XeType::Unknown, XeType::Unknown]),
            return_type: XeType::Unknown,
        }),
        "pop" => Some(FunctionSignature {
            params: Some(vec![XeType::Unknown]),
            return_type: XeType::Unknown,
        }),
        "keys" => Some(FunctionSignature {
            params: Some(vec![XeType::Unknown]),
            return_type: XeType::List(Box::new(XeType::Text)),
        }),
        "values" => Some(FunctionSignature {
            params: Some(vec![XeType::Unknown]),
            return_type: XeType::List(Box::new(XeType::Unknown)),
        }),
        "has_key" => Some(FunctionSignature {
            params: Some(vec![XeType::Unknown, XeType::Unknown]),
            return_type: XeType::Boolean,
        }),
        "contains" => Some(FunctionSignature {
            params: Some(vec![XeType::Unknown, XeType::Unknown]),
            return_type: XeType::Boolean,
        }),
        "split" => Some(FunctionSignature {
            params: Some(vec![XeType::Text, XeType::Text]),
            return_type: XeType::List(Box::new(XeType::Text)),
        }),
        "join" => Some(FunctionSignature {
            params: Some(vec![XeType::Unknown, XeType::Text]),
            return_type: XeType::Text,
        }),
        _ => None,
    }
}

pub struct SemanticAnalyzer {
    globals: HashMap<String, SymbolInfo>,
    global_order: Vec<String>,
    /// Block scopes of top-level code, or the scopes of the function being analyzed.
    scopes: Vec<HashMap<String, SymbolInfo>>,
    functions: HashMap<String, FunctionSignature>,
    structs: HashMap<String, Vec<String>>,
    loop_depth: usize,
    function_depth: usize,
    return_info: Option<ReturnInfo>,
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
            globals: HashMap::new(),
            global_order: Vec::new(),
            scopes: Vec::new(),
            functions,
            structs: HashMap::new(),
            loop_depth: 0,
            function_depth: 0,
            return_info: None,
        }
    }

    pub fn analyze(&mut self, program: &Program) -> XeResult<TypedProgram> {
        // Pass 1: Collect function signatures, structs and module-level variables
        for stmt in &program.statements {
            match &stmt.kind {
                StatementKind::FunctionDef { name, params, body } => {
                    if BUILTINS.iter().any(|(n, _)| n == name) {
                        return Err(XeError::new(
                            XeErrorKind::CannotRedefineBuiltin(name.clone()),
                            Some(stmt.span.clone()),
                        ));
                    }
                    if self.functions.contains_key(name) || self.structs.contains_key(name) {
                        return Err(XeError::new(
                            XeErrorKind::DuplicateFunction(name.clone()),
                            Some(stmt.span.clone()),
                        ));
                    }

                    self.functions.insert(name.clone(), FunctionSignature {
                        params: Some(vec![XeType::Unknown; params.len()]),
                        return_type: XeType::Unknown,
                    });

                    let mut declared_globals = Vec::new();
                    collect_global_names(body, &mut declared_globals);
                    for global in declared_globals {
                        self.declare_global_placeholder(&global, &stmt.span);
                    }
                }
                StatementKind::StructDef { name, fields } => {
                    if BUILTINS.iter().any(|(n, _)| n == name) {
                        return Err(XeError::new(
                            XeErrorKind::CannotRedefineBuiltin(name.clone()),
                            Some(stmt.span.clone()),
                        ));
                    }
                    if self.functions.contains_key(name) || self.structs.contains_key(name) {
                        return Err(XeError::new(
                            XeErrorKind::DuplicateFunction(name.clone()),
                            Some(stmt.span.clone()),
                        ));
                    }
                    check_unique_names(fields, &stmt.span)?;
                    self.structs.insert(name.clone(), fields.clone());
                    self.functions.insert(name.clone(), FunctionSignature {
                        params: Some(vec![XeType::Unknown; fields.len()]),
                        return_type: XeType::Struct(name.clone()),
                    });
                }
                StatementKind::Assignment { name, .. } => {
                    self.declare_global_placeholder(name, &stmt.span);
                }
                _ => {}
            }
        }

        // Pass 2: Full semantic analysis and IR production
        let mut typed_statements = Vec::new();
        for stmt in &program.statements {
            typed_statements.push(self.analyze_statement(stmt)?);
        }

        let globals = self
            .global_order
            .iter()
            .map(|name| {
                let info = &self.globals[name];
                let ty = if info.declared { info.ty.clone() } else { XeType::Unknown };
                (name.clone(), ty)
            })
            .collect();

        Ok(TypedProgram {
            statements: typed_statements,
            globals,
        })
    }

    fn analyze_statement(&mut self, stmt: &Statement) -> XeResult<TypedStatement> {
        let kind = match &stmt.kind {
            StatementKind::Import { .. } | StatementKind::FromImport { .. } => {
                // Imports are handled during linking
                TypedStatementKind::Nop
            }
            StatementKind::Global { names } => {
                if self.function_depth == 0 {
                    return Err(XeError::new(
                        XeErrorKind::GlobalOutsideFunction,
                        Some(stmt.span.clone()),
                    ));
                }
                for name in names {
                    self.declare_global_placeholder(name, &stmt.span);
                }
                TypedStatementKind::Nop
            }
            StatementKind::Assignment { name, value } => {
                let typed_value = self.analyze_expression(value)?;
                let typed_value = self.assign_variable(name, typed_value, &stmt.span)?;
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
                let typed_cond = self.analyze_condition(condition)?;
                let typed_then = self.analyze_block(then_block)?;
                let typed_else = match else_block {
                    Some(else_stmts) => Some(self.analyze_block(else_stmts)?),
                    None => None,
                };

                TypedStatementKind::If {
                    condition: typed_cond,
                    then_block: typed_then,
                    else_block: typed_else,
                }
            }
            StatementKind::While { condition, body } => {
                let typed_cond = self.analyze_condition(condition)?;
                self.loop_depth += 1;
                let typed_body = self.analyze_block(body);
                self.loop_depth -= 1;

                TypedStatementKind::While {
                    condition: typed_cond,
                    body: typed_body?,
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
                if typed_count.ty != XeType::Number {
                    typed_count = self.unwrap_to(typed_count, XeType::Number);
                }

                self.loop_depth += 1;
                let typed_body = self.analyze_block(body);
                self.loop_depth -= 1;

                TypedStatementKind::Repeat {
                    count: typed_count,
                    body: typed_body?,
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
                    XeType::Map => XeType::Text,
                    XeType::Struct(_) => XeType::Text,
                    XeType::Unknown => XeType::Unknown,
                    _ => {
                        return Err(XeError::new(
                            XeErrorKind::TypeMismatch {
                                expected: "list, text, or map".to_string(),
                                got: typed_iter.ty.name(),
                            },
                            Some(iterable.span.clone()),
                        ));
                    }
                };

                self.loop_depth += 1;
                self.push_scope();
                self.define_variable(variable, elem_ty, &stmt.span);
                let typed_body = body
                    .iter()
                    .map(|s| self.analyze_statement(s))
                    .collect::<XeResult<Vec<_>>>();
                self.pop_scope();
                self.loop_depth -= 1;

                TypedStatementKind::For {
                    variable: variable.clone(),
                    iterable: typed_iter,
                    body: typed_body?,
                }
            }
            StatementKind::FunctionDef {
                name,
                params,
                body,
            } => {
                check_unique_names(params, &stmt.span)?;

                self.function_depth += 1;
                let saved_scopes = std::mem::replace(&mut self.scopes, vec![HashMap::new()]);
                let saved_loop_depth = std::mem::replace(&mut self.loop_depth, 0);
                let saved_return_info = self.return_info.replace(ReturnInfo::default());

                let mut typed_params = Vec::new();
                for param in params {
                    self.define_variable(param, XeType::Unknown, &stmt.span);
                    typed_params.push((param.clone(), XeType::Unknown));
                }

                let typed_body = body
                    .iter()
                    .map(|s| self.analyze_statement(s))
                    .collect::<XeResult<Vec<_>>>();

                let return_info = self.return_info.take().unwrap_or_default();
                self.scopes = saved_scopes;
                self.loop_depth = saved_loop_depth;
                self.return_info = saved_return_info;
                self.function_depth -= 1;
                let typed_body = typed_body?;

                let return_type = infer_return_type(&return_info, block_always_returns(body));
                if let Some(sig) = self.functions.get_mut(name) {
                    sig.return_type = return_type.clone();
                }

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
                let typed_value = match value {
                    Some(expr) => {
                        let v = self.analyze_expression(expr)?;
                        if let Some(info) = &mut self.return_info {
                            info.value_types.push(v.ty.clone());
                        }
                        Some(v)
                    }
                    None => {
                        if let Some(info) = &mut self.return_info {
                            info.has_bare_return = true;
                        }
                        None
                    }
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
                let typed = self.analyze_expression(expr)?;
                // The value of a statement is discarded, so a `none` wrapper is unnecessary.
                let typed = match typed.kind {
                    TypedExpressionKind::Wrap(inner) if inner.ty == XeType::Void => *inner,
                    kind => TypedExpression { kind, ..typed },
                };
                TypedStatementKind::Expression(typed)
            }
            StatementKind::StructDef { name, fields } => {
                TypedStatementKind::StructDef {
                    name: name.clone(),
                    fields: fields.clone(),
                }
            }
            StatementKind::IndexAssignment { object, index, value } => {
                check_assignment_root(object)?;
                let typed_object = self.analyze_expression(object)?;
                let typed_index = self.analyze_expression(index)?;
                let typed_value = self.analyze_expression(value)?;

                let (typed_index, typed_value) = match &typed_object.ty {
                    XeType::List(inner) => {
                        let typed_index = self
                            .coerce(typed_index, &XeType::Number)
                            .map_err(|got| type_mismatch("number", got, &index.span))?;
                        let typed_value = self
                            .coerce(typed_value, inner)
                            .map_err(|got| type_mismatch(&inner.name(), got, &value.span))?;
                        (typed_index, typed_value)
                    }
                    ty if ty.is_dynamic() => (
                        self.coerce_to_dynamic(typed_index),
                        self.coerce_to_dynamic(typed_value),
                    ),
                    ty => {
                        return Err(XeError::new(
                            XeErrorKind::InvalidAssignmentTarget(format!(
                                "cannot assign to an element of a {} value",
                                ty
                            )),
                            Some(stmt.span.clone()),
                        ));
                    }
                };

                TypedStatementKind::IndexAssignment {
                    object: typed_object,
                    index: typed_index,
                    value: typed_value,
                }
            }
            StatementKind::FieldAssignment { object, field, value } => {
                check_assignment_root(object)?;
                let typed_object = self.analyze_expression(object)?;
                match &typed_object.ty {
                    XeType::Struct(struct_name) => {
                        let has_field = self
                            .structs
                            .get(struct_name)
                            .map(|fields| fields.contains(field))
                            .unwrap_or(true);
                        if !has_field {
                            return Err(XeError::new(
                                XeErrorKind::UndefinedVariable(format!("{}.{}", struct_name, field)),
                                Some(stmt.span.clone()),
                            ));
                        }
                    }
                    XeType::Map | XeType::Unknown => {}
                    ty => {
                        return Err(XeError::new(
                            XeErrorKind::InvalidAssignmentTarget(format!(
                                "cannot assign field '{}' on a {} value",
                                field, ty
                            )),
                            Some(stmt.span.clone()),
                        ));
                    }
                }
                let typed_value = self.analyze_expression(value)?;
                TypedStatementKind::FieldAssignment {
                    object: typed_object,
                    field: field.clone(),
                    value: self.coerce_to_dynamic(typed_value),
                }
            }
        };

        Ok(TypedStatement {
            kind,
            span: stmt.span.clone(),
        })
    }

    fn analyze_condition(&mut self, condition: &Expression) -> XeResult<TypedExpression> {
        let typed_cond = self.analyze_expression(condition)?;
        if !typed_cond.ty.is_compatible(&XeType::Boolean) {
            return Err(XeError::new(
                XeErrorKind::TypeMismatch {
                    expected: "boolean".to_string(),
                    got: typed_cond.ty.name(),
                },
                Some(condition.span.clone()),
            ));
        }
        if typed_cond.ty != XeType::Boolean {
            return Ok(self.unwrap_to(typed_cond, XeType::Boolean));
        }
        Ok(typed_cond)
    }

    fn analyze_block(&mut self, statements: &[Statement]) -> XeResult<Vec<TypedStatement>> {
        self.push_scope();
        let typed = statements
            .iter()
            .map(|s| self.analyze_statement(s))
            .collect::<XeResult<Vec<_>>>();
        self.pop_scope();
        typed
    }

    /// Assigns to a variable. The first assignment declares the variable and fixes its
    /// type; later assignments are checked against that type and converted to it.
    fn assign_variable(
        &mut self,
        name: &str,
        value: TypedExpression,
        span: &Span,
    ) -> XeResult<TypedExpression> {
        let existing = self
            .scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .or_else(|| self.globals.get(name))
            .cloned();

        match existing {
            Some(info) if info.declared => self.coerce(value, &info.ty).map_err(|got| {
                XeError::new(
                    XeErrorKind::TypeMismatch {
                        expected: format!("{} (defined at line {})", info.ty, info.defined_at.line),
                        got: got.name(),
                    },
                    Some(span.clone()),
                )
            }),
            Some(_) => {
                // First assignment of a known module-level variable.
                let global = self.globals.get_mut(name).expect("undeclared symbols are globals");
                global.ty = value.ty.clone();
                global.declared = true;
                global.defined_at = span.clone();
                Ok(value)
            }
            None => {
                let info = SymbolInfo {
                    ty: value.ty.clone(),
                    defined_at: span.clone(),
                    declared: true,
                };
                match self.scopes.last_mut() {
                    Some(scope) => {
                        scope.insert(name.to_string(), info);
                    }
                    None => {
                        self.global_order.push(name.to_string());
                        self.globals.insert(name.to_string(), info);
                    }
                }
                Ok(value)
            }
        }
    }

    /// Converts `value` to `target`, or returns the value's type if the two are incompatible.
    fn coerce(&self, value: TypedExpression, target: &XeType) -> Result<TypedExpression, XeType> {
        if value.ty == *target {
            Ok(value)
        } else if value.ty.is_compatible(target) {
            Ok(self.unwrap_to(value, target.clone()))
        } else {
            Err(value.ty)
        }
    }

    fn coerce_to_dynamic(&self, value: TypedExpression) -> TypedExpression {
        if value.ty.is_dynamic() {
            value
        } else {
            self.wrap_to_unknown(value)
        }
    }

    /// Compile-time checks for `append(list, item)` and `pop(list)`.
    fn check_list_mutation_args(
        &self,
        name: &str,
        typed_args: &[TypedExpression],
        args: &[Expression],
    ) -> XeResult<()> {
        let original_type = |arg: &TypedExpression| match &arg.kind {
            TypedExpressionKind::Wrap(inner) => inner.ty.clone(),
            _ => arg.ty.clone(),
        };

        let list_ty = original_type(&typed_args[0]);
        match &list_ty {
            XeType::List(inner) => {
                if name == "append" {
                    let item_ty = original_type(&typed_args[1]);
                    if !item_ty.is_compatible(inner) {
                        return Err(type_mismatch(&inner.name(), item_ty, &args[1].span));
                    }
                }
                Ok(())
            }
            XeType::Unknown => Ok(()),
            _ => Err(type_mismatch("list", list_ty, &args[0].span)),
        }
    }

    fn analyze_expression(&mut self, expr: &Expression) -> XeResult<TypedExpression> {
        let span = expr.span.clone();
        let (kind, ty) = match &expr.kind {
            ExpressionKind::Number(n) => (TypedExpressionKind::Number(*n), XeType::Number),
            ExpressionKind::String(s) => (TypedExpressionKind::String(s.clone()), XeType::Text),
            ExpressionKind::Boolean(b) => (TypedExpressionKind::Boolean(*b), XeType::Boolean),

            ExpressionKind::Identifier(name) => {
                if let Some(info) = self.get_symbol_info(name) {
                    let ty = if info.declared { info.ty.clone() } else { XeType::Unknown };
                    (TypedExpressionKind::Identifier(name.clone()), ty)
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

                if name == "append" || name == "pop" {
                    self.check_list_mutation_args(name, &typed_args, args)?;
                }

                (TypedExpressionKind::FunctionCall { name: name.clone(), args: typed_args }, sig.return_type)
            }

            ExpressionKind::Index { object, index } => {
                let obj_typed = self.analyze_expression(object)?;
                let mut idx_typed = self.analyze_expression(index)?;

                let ret_ty = match &obj_typed.ty {
                    XeType::List(inner) => {
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
                        *inner.clone()
                    }
                    XeType::Text => {
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
                        XeType::Text
                    }
                    XeType::Map => {
                        if idx_typed.ty != XeType::Unknown {
                            idx_typed = self.wrap_to_unknown(idx_typed);
                        }
                        XeType::Unknown
                    }
                    XeType::Struct(_) => {
                        if idx_typed.ty != XeType::Unknown {
                            idx_typed = self.wrap_to_unknown(idx_typed);
                        }
                        XeType::Unknown
                    }
                    XeType::Unknown => {
                        if idx_typed.ty != XeType::Unknown {
                            idx_typed = self.wrap_to_unknown(idx_typed);
                        }
                        XeType::Unknown
                    }
                    _ => return Err(XeError::new(
                        XeErrorKind::TypeMismatch {
                            expected: "list, text, or map".to_string(),
                            got: obj_typed.ty.name(),
                        },
                        Some(object.span.clone()),
                    )),
                };

                (TypedExpressionKind::Index { object: Box::new(obj_typed), index: Box::new(idx_typed) }, ret_ty)
            }

            ExpressionKind::Map(entries) => {
                let mut typed_entries = Vec::new();
                for (k, v) in entries {
                    let mut k_typed = self.analyze_expression(k)?;
                    let mut v_typed = self.analyze_expression(v)?;
                    if k_typed.ty != XeType::Unknown {
                        k_typed = self.wrap_to_unknown(k_typed);
                    }
                    if v_typed.ty != XeType::Unknown {
                        v_typed = self.wrap_to_unknown(v_typed);
                    }
                    typed_entries.push((k_typed, v_typed));
                }
                (TypedExpressionKind::Map(typed_entries), XeType::Map)
            }

            ExpressionKind::FieldAccess { object, field } => {
                let obj_typed = self.analyze_expression(object)?;
                if let XeType::Struct(struct_name) = &obj_typed.ty {
                    if let Some(fields) = self.structs.get(struct_name) {
                        if !fields.contains(field) {
                            return Err(XeError::new(
                                XeErrorKind::UndefinedVariable(format!("{}.{}", struct_name, field)),
                                Some(expr.span.clone()),
                            ));
                        }
                    }
                }
                (TypedExpressionKind::FieldAccess {
                    object: Box::new(obj_typed),
                    field: field.clone(),
                }, XeType::Unknown)
            }
        };

        let typed = TypedExpression { kind, ty, span };
        if typed.ty == XeType::Void {
            // A call to a function without a return value evaluates to `none`.
            return Ok(self.wrap_to_unknown(typed));
        }
        Ok(typed)
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

    fn define_variable(&mut self, name: &str, ty: XeType, span: &Span) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), SymbolInfo {
                ty,
                defined_at: span.clone(),
                declared: true,
            });
        }
    }

    fn declare_global_placeholder(&mut self, name: &str, span: &Span) {
        if !self.globals.contains_key(name) {
            self.global_order.push(name.to_string());
            self.globals.insert(name.to_string(), SymbolInfo {
                ty: XeType::Unknown,
                defined_at: span.clone(),
                declared: false,
            });
        }
    }

    fn get_symbol_info(&self, name: &str) -> Option<&SymbolInfo> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name))
            .or_else(|| self.globals.get(name))
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

fn type_mismatch(expected: &str, got: XeType, span: &Span) -> XeError {
    XeError::new(
        XeErrorKind::TypeMismatch {
            expected: expected.to_string(),
            got: got.name(),
        },
        Some(span.clone()),
    )
}

/// A function's return type: native when every path returns a value of the same type,
/// dynamic when paths disagree or may return `none`, and void when no value is returned.
fn infer_return_type(info: &ReturnInfo, always_returns: bool) -> XeType {
    let Some(first) = info.value_types.first() else {
        return XeType::Void;
    };
    let all_same = info.value_types.iter().all(|ty| ty == first);
    if info.has_bare_return || !always_returns || !all_same {
        XeType::Unknown
    } else {
        first.clone()
    }
}

fn check_unique_names(names: &[String], span: &Span) -> XeResult<()> {
    for (i, name) in names.iter().enumerate() {
        if names[..i].contains(name) {
            return Err(XeError::new(
                XeErrorKind::DuplicateParameter(name.clone()),
                Some(span.clone()),
            ));
        }
    }
    Ok(())
}

/// Element and field assignments must write into a variable, e.g. `grid[0][1] = 5`.
fn check_assignment_root(object: &Expression) -> XeResult<()> {
    match &object.kind {
        ExpressionKind::Identifier(_) => Ok(()),
        ExpressionKind::Index { object, .. } | ExpressionKind::FieldAccess { object, .. } => {
            check_assignment_root(object)
        }
        _ => Err(XeError::new(
            XeErrorKind::InvalidAssignmentTarget(
                "can only assign to elements or fields of a variable".to_string(),
            ),
            Some(object.span.clone()),
        )),
    }
}

fn collect_global_names(statements: &[Statement], names: &mut Vec<String>) {
    for statement in statements {
        match &statement.kind {
            StatementKind::Global { names: declared } => names.extend(declared.iter().cloned()),
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                collect_global_names(then_block, names);
                if let Some(else_block) = else_block {
                    collect_global_names(else_block, names);
                }
            }
            StatementKind::While { body, .. }
            | StatementKind::Repeat { body, .. }
            | StatementKind::For { body, .. } => collect_global_names(body, names),
            _ => {}
        }
    }
}
