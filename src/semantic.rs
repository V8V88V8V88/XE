use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::builtins::Builtin;
use crate::error::{Span, XeError, XeErrorKind, XeResult};

/// Analyzes a linked program and produces the typed program.
pub fn analyze_program(program: &Program) -> XeResult<TypedProgram> {
    SemanticAnalyzer::new().analyze(program)
}

#[derive(Clone)]
struct SymbolInfo {
    ty: XeType,
    defined_at: Span,
    /// False for a module-level variable that is known to exist but whose first
    /// assignment (which fixes its type) has not been analyzed yet.
    declared: bool,
}

struct FunctionInfo {
    params: Vec<XeType>,
    return_type: XeType,
}

#[derive(Clone, Copy, PartialEq)]
enum ScopeKind {
    /// Parameters and every variable assigned in a function body.
    FunctionLocals,
    /// A `for` loop variable.
    Loop,
    /// Lambda parameters.
    Lambda,
}

struct Scope {
    kind: ScopeKind,
    vars: HashMap<String, SymbolInfo>,
}

/// Return statements seen while analyzing the current function body.
#[derive(Default)]
struct ReturnInfo {
    value_types: Vec<XeType>,
    has_bare_return: bool,
}

struct FunctionState {
    returns: ReturnInfo,
    locals: Vec<(String, XeType)>,
    /// Locals definitely assigned at the current point; `None` when unreachable.
    assigned: Option<HashSet<String>>,
}

struct LambdaFrame {
    scope_index: usize,
    captures: Vec<(String, XeType)>,
}

enum VarLocation {
    Local(usize),
    Global,
}

pub struct SemanticAnalyzer {
    globals: HashMap<String, SymbolInfo>,
    global_order: Vec<String>,
    scopes: Vec<Scope>,
    functions: HashMap<String, FunctionInfo>,
    structs: HashMap<String, Vec<String>>,
    loop_depth: usize,
    function: Option<FunctionState>,
    lambdas: Vec<LambdaFrame>,
}

impl SemanticAnalyzer {
    fn new() -> Self {
        Self {
            globals: HashMap::new(),
            global_order: Vec::new(),
            scopes: Vec::new(),
            functions: HashMap::new(),
            structs: HashMap::new(),
            loop_depth: 0,
            function: None,
            lambdas: Vec::new(),
        }
    }

    fn analyze(&mut self, program: &Program) -> XeResult<TypedProgram> {
        // Pass 1: Collect function signatures, structs and module-level variables
        for stmt in &program.statements {
            match &stmt.kind {
                StatementKind::FunctionDef { name, params, body } => {
                    self.check_new_definition(name, &stmt.span)?;
                    self.functions.insert(name.clone(), FunctionInfo {
                        params: vec![XeType::Unknown; params.len()],
                        return_type: XeType::Unknown,
                    });

                    let mut declared_globals = Vec::new();
                    collect_global_names(body, &mut declared_globals);
                    for global in declared_globals {
                        self.declare_global_placeholder(&global, &stmt.span);
                    }
                }
                StatementKind::StructDef { name, fields } => {
                    self.check_new_definition(name, &stmt.span)?;
                    check_unique_names(fields, &stmt.span)?;
                    self.structs.insert(name.clone(), fields.clone());
                    self.functions.insert(name.clone(), FunctionInfo {
                        params: vec![XeType::Unknown; fields.len()],
                        return_type: XeType::Struct(name.clone()),
                    });
                }
                _ => {
                    let mut targets = Vec::new();
                    collect_top_level_targets(stmt, &mut targets);
                    for name in targets {
                        self.declare_global_placeholder(&name, &stmt.span);
                    }
                }
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

    fn check_new_definition(&self, name: &str, span: &Span) -> XeResult<()> {
        if self.functions.contains_key(name) || self.structs.contains_key(name) {
            return Err(XeError::new(
                XeErrorKind::DuplicateFunction(name.to_string()),
                Some(span.clone()),
            ));
        }
        Ok(())
    }

    fn analyze_statements(&mut self, statements: &[Statement]) -> XeResult<Vec<TypedStatement>> {
        statements.iter().map(|s| self.analyze_statement(s)).collect()
    }

    fn analyze_statement(&mut self, stmt: &Statement) -> XeResult<TypedStatement> {
        let kind = match &stmt.kind {
            StatementKind::Import { .. } | StatementKind::FromImport { .. } | StatementKind::Pass => {
                TypedStatementKind::Nop
            }
            StatementKind::Global { names } => {
                if self.function.is_none() {
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
                let condition = self.analyze_condition(condition)?;
                let before = self.flow();
                let then_block = self.analyze_statements(then_block)?;
                let after_then = self.flow();
                self.set_flow(before);
                let else_block = match else_block {
                    Some(block) => Some(self.analyze_statements(block)?),
                    None => None,
                };
                let after_else = self.flow();
                self.set_flow(meet(after_then, after_else));

                TypedStatementKind::If {
                    condition,
                    then_block,
                    else_block,
                }
            }
            StatementKind::While { condition, body } => {
                let condition = self.analyze_condition(condition)?;
                let body = self.analyze_loop_body(body, None)?;
                TypedStatementKind::While { condition, body }
            }
            StatementKind::Repeat { count, body } => {
                let count = self.analyze_expression(count)?;
                let count = self
                    .coerce(count, &XeType::Number)
                    .map_err(|got| type_mismatch("number", got, &stmt.span))?;
                let body = self.analyze_loop_body(body, None)?;
                TypedStatementKind::Repeat { count, body }
            }
            StatementKind::For {
                variable,
                iterable,
                body,
            } => {
                let typed_iter = self.analyze_expression(iterable)?;
                let is_range = matches!(
                    typed_iter.kind,
                    TypedExpressionKind::BuiltinCall {
                        builtin: Builtin::Range,
                        ..
                    }
                );
                let variable_type = match &typed_iter.ty {
                    _ if is_range => XeType::Number,
                    XeType::Text | XeType::Struct(_) => XeType::Text,
                    XeType::List | XeType::Map | XeType::Unknown => XeType::Unknown,
                    other => {
                        return Err(XeError::new(
                            XeErrorKind::TypeMismatch {
                                expected: "list, text, or map".to_string(),
                                got: other.name(),
                            },
                            Some(iterable.span.clone()),
                        ));
                    }
                };
                let body = self.analyze_loop_body(
                    body,
                    Some((variable.clone(), variable_type.clone(), stmt.span.clone())),
                )?;

                TypedStatementKind::For {
                    variable: variable.clone(),
                    variable_type,
                    iterable: typed_iter,
                    body,
                }
            }
            StatementKind::FunctionDef { name, params, body } => {
                self.analyze_function(name, params, body, &stmt.span)?
            }
            StatementKind::StructDef { name, fields } => TypedStatementKind::StructDef {
                name: name.clone(),
                fields: fields.clone(),
            },
            StatementKind::Return { value } => {
                if self.function.is_none() {
                    return Err(XeError::new(
                        XeErrorKind::ReturnOutsideFunction,
                        Some(stmt.span.clone()),
                    ));
                }
                let typed_value = match value {
                    Some(expr) => Some(self.analyze_expression(expr)?),
                    None => None,
                };
                if let Some(function) = &mut self.function {
                    match &typed_value {
                        Some(v) => function.returns.value_types.push(v.ty.clone()),
                        None => function.returns.has_bare_return = true,
                    }
                }
                self.set_flow(None);
                TypedStatementKind::Return { value: typed_value }
            }
            StatementKind::Break | StatementKind::Continue => {
                let is_break = matches!(stmt.kind, StatementKind::Break);
                if self.loop_depth == 0 {
                    let kind = if is_break {
                        XeErrorKind::BreakOutsideLoop
                    } else {
                        XeErrorKind::ContinueOutsideLoop
                    };
                    return Err(XeError::new(kind, Some(stmt.span.clone())));
                }
                self.set_flow(None);
                if is_break {
                    TypedStatementKind::Break
                } else {
                    TypedStatementKind::Continue
                }
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
            StatementKind::IndexAssignment {
                object,
                index,
                value,
                op,
            } => {
                let typed_value = self.analyze_expression(value)?;
                let typed_object = self.analyze_expression(object)?;
                let typed_index = self.analyze_expression(index)?;
                match &typed_object.ty {
                    XeType::List => {
                        if !typed_index.ty.is_compatible(&XeType::Number) {
                            return Err(type_mismatch("number", typed_index.ty, &index.span));
                        }
                    }
                    XeType::Map | XeType::Struct(_) | XeType::Unknown => {}
                    ty => {
                        return Err(XeError::new(
                            XeErrorKind::InvalidAssignmentTarget(format!(
                                "cannot assign to an element of a {} value",
                                ty
                            )),
                            Some(stmt.span.clone()),
                        ));
                    }
                }
                TypedStatementKind::IndexAssignment {
                    object: typed_object,
                    index: self.coerce_to_dynamic(typed_index),
                    value: self.coerce_to_dynamic(typed_value),
                    op: *op,
                }
            }
            StatementKind::FieldAssignment {
                object,
                field,
                value,
                op,
            } => {
                let typed_value = self.analyze_expression(value)?;
                let typed_object = self.analyze_expression(object)?;
                self.check_field_access(&typed_object, field, &stmt.span, true)?;
                TypedStatementKind::FieldAssignment {
                    object: typed_object,
                    field: field.clone(),
                    value: self.coerce_to_dynamic(typed_value),
                    op: *op,
                }
            }
            StatementKind::Try {
                body,
                catch_variable,
                handler,
            } => {
                let before = self.flow();
                let body = self.analyze_statements(body)?;
                let after_body = self.flow();
                // The handler can start after any statement of the body has failed.
                self.set_flow(before);
                if let Some(name) = catch_variable {
                    let message = TypedExpression {
                        kind: TypedExpressionKind::String(String::new()),
                        ty: XeType::Text,
                        span: stmt.span.clone(),
                    };
                    self.assign_variable(name, message, &stmt.span)?;
                }
                let handler = self.analyze_statements(handler)?;
                let after_handler = self.flow();
                self.set_flow(meet(after_body, after_handler));
                TypedStatementKind::Try {
                    body,
                    catch_variable: catch_variable.clone(),
                    handler,
                }
            }
        };

        Ok(TypedStatement {
            kind,
            span: stmt.span.clone(),
        })
    }

    /// Analyzes a loop body. Assignments inside it are not definite after the loop,
    /// since the body may run zero times.
    fn analyze_loop_body(
        &mut self,
        body: &[Statement],
        variable: Option<(String, XeType, Span)>,
    ) -> XeResult<Vec<TypedStatement>> {
        let before = self.flow();
        self.loop_depth += 1;
        let has_variable = variable.is_some();
        if let Some((name, ty, span)) = variable {
            let mut vars = HashMap::new();
            vars.insert(name, SymbolInfo {
                ty,
                defined_at: span,
                declared: true,
            });
            self.scopes.push(Scope {
                kind: ScopeKind::Loop,
                vars,
            });
        }
        let typed = self.analyze_statements(body);
        if has_variable {
            self.scopes.pop();
        }
        self.loop_depth -= 1;
        self.set_flow(before);
        typed
    }

    fn analyze_function(
        &mut self,
        name: &str,
        params: &[String],
        body: &[Statement],
        span: &Span,
    ) -> XeResult<TypedStatementKind> {
        check_unique_names(params, span)?;
        let param_types = self.functions[name].params.clone();

        let mut vars = HashMap::new();
        for (param, ty) in params.iter().zip(&param_types) {
            vars.insert(param.clone(), SymbolInfo {
                ty: ty.clone(),
                defined_at: span.clone(),
                declared: true,
            });
        }

        let saved_scopes = std::mem::replace(
            &mut self.scopes,
            vec![Scope {
                kind: ScopeKind::FunctionLocals,
                vars,
            }],
        );
        let saved_loop_depth = std::mem::replace(&mut self.loop_depth, 0);
        let saved_lambdas = std::mem::take(&mut self.lambdas);
        let saved_function = self.function.replace(FunctionState {
            returns: ReturnInfo::default(),
            locals: Vec::new(),
            assigned: Some(params.iter().cloned().collect()),
        });

        let typed_body = self.analyze_statements(body);

        let state = self.function.take().expect("function state is set");
        self.function = saved_function;
        self.lambdas = saved_lambdas;
        self.loop_depth = saved_loop_depth;
        self.scopes = saved_scopes;
        let typed_body = typed_body?;

        let always_returns = block_always_returns(body);
        let return_type = infer_return_type(&state.returns, always_returns);
        if let Some(info) = self.functions.get_mut(name) {
            info.return_type = return_type.clone();
        }

        Ok(TypedStatementKind::FunctionDef {
            name: name.to_string(),
            params: params.iter().cloned().zip(param_types).collect(),
            locals: state.locals,
            body: typed_body,
            return_type,
        })
    }

    fn analyze_condition(&mut self, condition: &Expression) -> XeResult<TypedExpression> {
        let typed = self.analyze_expression(condition)?;
        Ok(self.truthy(typed))
    }

    /// Any value can be used as a condition; non-booleans use their truthiness.
    fn truthy(&self, typed: TypedExpression) -> TypedExpression {
        if typed.ty == XeType::Boolean {
            return typed;
        }
        TypedExpression {
            ty: XeType::Boolean,
            span: typed.span.clone(),
            kind: TypedExpressionKind::Truthy(Box::new(typed)),
        }
    }

    // --- Definite assignment of function locals ---

    fn flow(&self) -> Option<HashSet<String>> {
        self.function.as_ref().and_then(|f| f.assigned.clone())
    }

    fn set_flow(&mut self, state: Option<HashSet<String>>) {
        if let Some(function) = &mut self.function {
            function.assigned = state;
        }
    }

    fn mark_assigned(&mut self, name: &str) {
        if let Some(function) = &mut self.function {
            if let Some(assigned) = &mut function.assigned {
                assigned.insert(name.to_string());
            }
        }
    }

    fn is_definitely_assigned(&self, name: &str) -> bool {
        match &self.function {
            Some(function) => match &function.assigned {
                Some(assigned) => assigned.contains(name),
                None => true, // unreachable code
            },
            None => true,
        }
    }

    // --- Variables ---

    fn find_variable(&self, name: &str) -> Option<(VarLocation, SymbolInfo)> {
        for (index, scope) in self.scopes.iter().enumerate().rev() {
            if let Some(info) = scope.vars.get(name) {
                return Some((VarLocation::Local(index), info.clone()));
            }
        }
        self.globals
            .get(name)
            .map(|info| (VarLocation::Global, info.clone()))
    }

    /// Resolves a variable read, recording lambda captures and checking that function
    /// locals are definitely assigned.
    fn read_variable(&mut self, name: &str, span: &Span) -> XeResult<Option<XeType>> {
        let Some((location, info)) = self.find_variable(name) else {
            return Ok(None);
        };
        match location {
            VarLocation::Global => Ok(Some(if info.declared { info.ty } else { XeType::Unknown })),
            VarLocation::Local(index) => {
                if self.scopes[index].kind == ScopeKind::FunctionLocals
                    && !self.is_definitely_assigned(name)
                {
                    return Err(XeError::new(
                        XeErrorKind::MaybeUnassigned(name.to_string()),
                        Some(span.clone()),
                    ));
                }
                for frame in &mut self.lambdas {
                    if index < frame.scope_index && !frame.captures.iter().any(|(n, _)| n == name) {
                        frame.captures.push((name.to_string(), info.ty.clone()));
                    }
                }
                Ok(Some(info.ty))
            }
        }
    }

    /// Assigns to a variable. The first assignment declares the variable and fixes its
    /// type; later assignments are checked against that type and converted to it.
    fn assign_variable(
        &mut self,
        name: &str,
        value: TypedExpression,
        span: &Span,
    ) -> XeResult<TypedExpression> {
        let value = match value.ty {
            XeType::Void => self.wrap_to_unknown(value),
            _ => value,
        };

        match self.find_variable(name) {
            Some((location, info)) if info.declared => {
                let coerced = self.coerce(value, &info.ty).map_err(|got| {
                    XeError::new(
                        XeErrorKind::TypeMismatch {
                            expected: format!("{} (defined at line {})", info.ty, info.defined_at.line),
                            got: got.name(),
                        },
                        Some(span.clone()),
                    )
                })?;
                if let VarLocation::Local(index) = location {
                    if self.scopes[index].kind == ScopeKind::FunctionLocals {
                        self.mark_assigned(name);
                    }
                }
                Ok(coerced)
            }
            Some((VarLocation::Global, _)) => {
                // First assignment of a known module-level variable.
                let global = self.globals.get_mut(name).expect("found above");
                global.ty = value.ty.clone();
                global.declared = true;
                global.defined_at = span.clone();
                Ok(value)
            }
            Some((VarLocation::Local(_), _)) => unreachable!("locals are always declared"),
            None => {
                let info = SymbolInfo {
                    ty: value.ty.clone(),
                    defined_at: span.clone(),
                    declared: true,
                };
                match &mut self.function {
                    Some(function) => {
                        function.locals.push((name.to_string(), value.ty.clone()));
                        self.scopes[0].vars.insert(name.to_string(), info);
                        self.mark_assigned(name);
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

    // --- Expressions ---

    fn analyze_expression(&mut self, expr: &Expression) -> XeResult<TypedExpression> {
        let span = expr.span.clone();
        let (kind, ty) = match &expr.kind {
            ExpressionKind::Number(n) => (TypedExpressionKind::Number(*n), XeType::Number),
            ExpressionKind::String(s) => (TypedExpressionKind::String(s.clone()), XeType::Text),
            ExpressionKind::Boolean(b) => (TypedExpressionKind::Boolean(*b), XeType::Boolean),
            ExpressionKind::None => (TypedExpressionKind::None, XeType::Unknown),

            ExpressionKind::Identifier(name) => {
                if let Some(ty) = self.read_variable(name, &span)? {
                    (TypedExpressionKind::Identifier(name.clone()), ty)
                } else if self.functions.contains_key(name) {
                    (TypedExpressionKind::FunctionRef(name.clone()), XeType::Function)
                } else if let Some(builtin) = Builtin::from_name(name) {
                    (TypedExpressionKind::BuiltinRef(builtin), XeType::Function)
                } else {
                    return Err(XeError::new(
                        XeErrorKind::UndefinedVariable(name.clone()),
                        Some(span),
                    ));
                }
            }

            ExpressionKind::List(elements) => {
                let elements = elements
                    .iter()
                    .map(|e| {
                        let typed = self.analyze_expression(e)?;
                        Ok(self.coerce_to_dynamic(typed))
                    })
                    .collect::<XeResult<Vec<_>>>()?;
                (TypedExpressionKind::List(elements), XeType::List)
            }

            ExpressionKind::Map(entries) => {
                let entries = entries
                    .iter()
                    .map(|(k, v)| {
                        let k = self.analyze_expression(k)?;
                        let v = self.analyze_expression(v)?;
                        Ok((self.coerce_to_dynamic(k), self.coerce_to_dynamic(v)))
                    })
                    .collect::<XeResult<Vec<_>>>()?;
                (TypedExpressionKind::Map(entries), XeType::Map)
            }

            ExpressionKind::BinaryOp { left, op, right } => {
                let l = self.analyze_expression(left)?;
                let r = self.analyze_expression(right)?;
                self.binary_op(l, *op, r, &span)?
            }

            ExpressionKind::UnaryOp { op, operand } => {
                let o = self.analyze_expression(operand)?;
                match op {
                    UnaryOperator::Negate => {
                        let o = self
                            .coerce(o, &XeType::Number)
                            .map_err(|got| type_mismatch("number", got, &span))?;
                        (
                            TypedExpressionKind::UnaryOp {
                                op: *op,
                                operand: Box::new(o),
                            },
                            XeType::Number,
                        )
                    }
                    UnaryOperator::Not => (
                        TypedExpressionKind::UnaryOp {
                            op: *op,
                            operand: Box::new(self.truthy(o)),
                        },
                        XeType::Boolean,
                    ),
                }
            }

            ExpressionKind::FunctionCall { name, args } => {
                if self.find_variable(name).is_some() {
                    let callee = self.analyze_expression(&Expression {
                        kind: ExpressionKind::Identifier(name.clone()),
                        span: span.clone(),
                    })?;
                    self.dynamic_call(callee, args, &format!("'{}'", name))?
                } else {
                    self.named_call(name, None, args, &span)?
                }
            }

            ExpressionKind::MethodCall {
                object,
                method,
                args,
            } => {
                if self.functions.contains_key(method) || Builtin::is_builtin(method) {
                    // `object.method(args)` is `method(object, args)`.
                    self.named_call(method, Some(object), args, &span)?
                } else {
                    let callee = self.analyze_expression(&Expression {
                        kind: ExpressionKind::FieldAccess {
                            object: object.clone(),
                            field: method.clone(),
                        },
                        span: span.clone(),
                    })?;
                    self.dynamic_call(callee, args, &format!("'{}'", method))?
                }
            }

            ExpressionKind::Call { callee, args } => {
                let callee = self.analyze_expression(callee)?;
                self.dynamic_call(callee, args, "this value")?
            }

            ExpressionKind::Lambda { params, body } => {
                check_unique_names(params, &span)?;
                let vars = params
                    .iter()
                    .map(|p| {
                        (p.clone(), SymbolInfo {
                            ty: XeType::Unknown,
                            defined_at: span.clone(),
                            declared: true,
                        })
                    })
                    .collect();
                self.scopes.push(Scope {
                    kind: ScopeKind::Lambda,
                    vars,
                });
                self.lambdas.push(LambdaFrame {
                    scope_index: self.scopes.len() - 1,
                    captures: Vec::new(),
                });
                let body = self.analyze_expression(body);
                let frame = self.lambdas.pop().expect("pushed above");
                self.scopes.pop();
                let body = self.coerce_to_dynamic(body?);
                (
                    TypedExpressionKind::Lambda {
                        params: params.clone(),
                        captures: frame.captures,
                        body: Box::new(body),
                    },
                    XeType::Function,
                )
            }

            ExpressionKind::Index { object, index } => {
                let obj = self.analyze_expression(object)?;
                let idx = self.analyze_expression(index)?;
                let ty = match &obj.ty {
                    XeType::Text => XeType::Text,
                    XeType::List | XeType::Map | XeType::Struct(_) | XeType::Unknown => XeType::Unknown,
                    other => {
                        return Err(type_mismatch("list, text, or map", other.clone(), &object.span));
                    }
                };
                if matches!(obj.ty, XeType::Text | XeType::List) && !idx.ty.is_compatible(&XeType::Number) {
                    return Err(type_mismatch("number", idx.ty, &index.span));
                }
                (
                    TypedExpressionKind::Index {
                        object: Box::new(obj),
                        index: Box::new(self.coerce_to_dynamic(idx)),
                    },
                    ty,
                )
            }

            ExpressionKind::Slice { object, start, end } => {
                let obj = self.analyze_expression(object)?;
                let ty = match &obj.ty {
                    XeType::Text | XeType::List | XeType::Unknown => obj.ty.clone(),
                    other => return Err(type_mismatch("list or text", other.clone(), &object.span)),
                };
                let start = self.slice_bound(start)?;
                let end = self.slice_bound(end)?;
                (
                    TypedExpressionKind::Slice {
                        object: Box::new(obj),
                        start,
                        end,
                    },
                    ty,
                )
            }

            ExpressionKind::FieldAccess { object, field } => {
                let obj = self.analyze_expression(object)?;
                self.check_field_access(&obj, field, &expr.span, false)?;
                (
                    TypedExpressionKind::FieldAccess {
                        object: Box::new(obj),
                        field: field.clone(),
                    },
                    XeType::Unknown,
                )
            }
        };

        let typed = TypedExpression { kind, ty, span };
        if typed.ty == XeType::Void {
            // A call to a function without a return value evaluates to `none`.
            return Ok(self.wrap_to_unknown(typed));
        }
        Ok(typed)
    }

    fn slice_bound(&mut self, bound: &Option<Box<Expression>>) -> XeResult<Option<Box<TypedExpression>>> {
        let Some(bound) = bound else {
            return Ok(None);
        };
        let typed = self.analyze_expression(bound)?;
        let typed = self
            .coerce(typed, &XeType::Number)
            .map_err(|got| type_mismatch("number", got, &bound.span))?;
        Ok(Some(Box::new(typed)))
    }

    fn check_field_access(
        &self,
        object: &TypedExpression,
        field: &str,
        span: &Span,
        is_assignment: bool,
    ) -> XeResult<()> {
        match &object.ty {
            XeType::Struct(struct_name) => {
                let has_field = self
                    .structs
                    .get(struct_name)
                    .map(|fields| fields.iter().any(|f| f == field))
                    .unwrap_or(true);
                if has_field {
                    Ok(())
                } else {
                    Err(XeError::new(
                        XeErrorKind::UndefinedVariable(format!("{}.{}", struct_name, field)),
                        Some(span.clone()),
                    ))
                }
            }
            XeType::Map | XeType::Unknown => Ok(()),
            ty => {
                let message = if is_assignment {
                    format!("cannot assign field '{}' on a {} value", field, ty)
                } else {
                    format!("cannot access field '{}' on a {} value", field, ty)
                };
                Err(XeError::new(
                    XeErrorKind::InvalidAssignmentTarget(message),
                    Some(span.clone()),
                ))
            }
        }
    }

    fn binary_op(
        &self,
        l: TypedExpression,
        op: BinaryOperator,
        r: TypedExpression,
        span: &Span,
    ) -> XeResult<(TypedExpressionKind, XeType)> {
        use XeType::{Boolean, List, Number, Text, Unknown};
        let mismatch = |l: &XeType, r: &XeType| {
            Err(XeError::new(
                XeErrorKind::InvalidOperation(format!(
                    "operator '{}' is not defined for {} and {}",
                    op.symbol(),
                    l,
                    r
                )),
                Some(span.clone()),
            ))
        };
        let node = |l: TypedExpression, r: TypedExpression| TypedExpressionKind::BinaryOp {
            left: Box::new(l),
            op,
            right: Box::new(r),
        };

        Ok(match op {
            BinaryOperator::Add => match (&l.ty, &r.ty) {
                (Number, Number) => (node(l, r), Number),
                (Text, Text) => (node(l, r), Text),
                // Text joined with any other value converts that value to text.
                (Text, _) | (_, Text) => (
                    node(self.coerce_to_dynamic(l), self.coerce_to_dynamic(r)),
                    Text,
                ),
                (List, List) => (node(l, r), List),
                (Unknown, Number | List | Unknown) | (Number | List, Unknown) => (
                    node(self.coerce_to_dynamic(l), self.coerce_to_dynamic(r)),
                    Unknown,
                ),
                (lt, rt) => return mismatch(lt, rt),
            },
            BinaryOperator::Subtract
            | BinaryOperator::Multiply
            | BinaryOperator::Divide
            | BinaryOperator::FloorDivide
            | BinaryOperator::Modulo
            | BinaryOperator::Power => {
                if !l.ty.is_compatible(&Number) || !r.ty.is_compatible(&Number) {
                    return mismatch(&l.ty, &r.ty);
                }
                let l = self.coerce(l, &Number).unwrap_or_else(|_| unreachable!());
                let r = self.coerce(r, &Number).unwrap_or_else(|_| unreachable!());
                (node(l, r), Number)
            }
            BinaryOperator::Less
            | BinaryOperator::Greater
            | BinaryOperator::LessEqual
            | BinaryOperator::GreaterEqual => {
                let target = match (&l.ty, &r.ty) {
                    (Number, Number | Unknown) | (Unknown, Number) => Number,
                    (Text, Text | Unknown) | (Unknown, Text) => Text,
                    (Unknown, Unknown) => Unknown,
                    (lt, rt) => return mismatch(lt, rt),
                };
                let l = self.coerce(l, &target).unwrap_or_else(|_| unreachable!());
                let r = self.coerce(r, &target).unwrap_or_else(|_| unreachable!());
                (node(l, r), Boolean)
            }
            BinaryOperator::Equal | BinaryOperator::NotEqual => (node(l, r), Boolean),
            BinaryOperator::In | BinaryOperator::NotIn => {
                if !matches!(r.ty, List | Text | XeType::Map | XeType::Struct(_) | Unknown) {
                    return mismatch(&l.ty, &r.ty);
                }
                (node(self.coerce_to_dynamic(l), self.coerce_to_dynamic(r)), Boolean)
            }
            BinaryOperator::And | BinaryOperator::Or => (node(self.truthy(l), self.truthy(r)), Boolean),
        })
    }

    /// Calls `name(receiver?, args...)`: a user function, struct constructor, or builtin.
    fn named_call(
        &mut self,
        name: &str,
        receiver: Option<&Expression>,
        args: &[Expression],
        span: &Span,
    ) -> XeResult<(TypedExpressionKind, XeType)> {
        let all_args: Vec<&Expression> = receiver.into_iter().chain(args).collect();

        if let Some(info) = self.functions.get(name) {
            let param_types = info.params.clone();
            let return_type = info.return_type.clone();
            if all_args.len() != param_types.len() {
                return Err(XeError::new(
                    XeErrorKind::WrongArgumentCount {
                        name: name.to_string(),
                        expected: param_types.len(),
                        got: all_args.len(),
                    },
                    Some(span.clone()),
                ));
            }
            let mut typed_args = Vec::new();
            for (arg, param_ty) in all_args.iter().zip(&param_types) {
                let typed = self.analyze_expression(arg)?;
                let typed = self
                    .coerce(typed, param_ty)
                    .map_err(|got| type_mismatch(&param_ty.name(), got, &arg.span))?;
                typed_args.push(typed);
            }
            return Ok((
                TypedExpressionKind::FunctionCall {
                    name: name.to_string(),
                    args: typed_args,
                },
                return_type,
            ));
        }

        let Some(builtin) = Builtin::from_name(name) else {
            return Err(XeError::new(
                XeErrorKind::UndefinedFunction(name.to_string()),
                Some(span.clone()),
            ));
        };
        let signature = builtin.signature();
        if !signature.accepts(all_args.len()) {
            return Err(XeError::new(
                XeErrorKind::InvalidOperation(format!(
                    "function '{}' expects {} arguments, got {}",
                    name,
                    signature.describe_arity(),
                    all_args.len()
                )),
                Some(span.clone()),
            ));
        }

        let mut original_types = Vec::new();
        let mut typed_args = Vec::new();
        for (i, arg) in all_args.iter().enumerate() {
            let typed = self.analyze_expression(arg)?;
            original_types.push(typed.ty.clone());
            let expected = signature.param_type(i);
            if !typed.ty.is_compatible(&expected) {
                return Err(type_mismatch(&expected.name(), typed.ty, &arg.span));
            }
            let typed = if expected.is_dynamic() {
                self.coerce_to_dynamic(typed)
            } else {
                self.coerce(typed, &expected).unwrap_or_else(|_| unreachable!())
            };
            typed_args.push(typed);
        }

        let return_type = match builtin {
            Builtin::Convert => match &all_args[1].kind {
                ExpressionKind::String(target) => match target.as_str() {
                    "number" => XeType::Number,
                    "text" => XeType::Text,
                    "boolean" => XeType::Boolean,
                    other => {
                        return Err(XeError::new(
                            XeErrorKind::InvalidOperation(format!(
                                "convert() target must be \"number\", \"text\" or \"boolean\", got \"{}\"",
                                other
                            )),
                            Some(all_args[1].span.clone()),
                        ));
                    }
                },
                _ => XeType::Unknown,
            },
            Builtin::Min | Builtin::Max
                if original_types.len() >= 2 && original_types.iter().all(|t| *t == XeType::Number) =>
            {
                XeType::Number
            }
            _ => signature.return_type,
        };

        Ok((
            TypedExpressionKind::BuiltinCall {
                builtin,
                args: typed_args,
            },
            return_type,
        ))
    }

    fn dynamic_call(
        &mut self,
        callee: TypedExpression,
        args: &[Expression],
        description: &str,
    ) -> XeResult<(TypedExpressionKind, XeType)> {
        if !callee.ty.is_compatible(&XeType::Function) {
            return Err(XeError::new(
                XeErrorKind::NotCallable(format!("{} (a {} value)", description, callee.ty)),
                Some(callee.span.clone()),
            ));
        }
        let args = args
            .iter()
            .map(|arg| {
                let typed = self.analyze_expression(arg)?;
                Ok(self.coerce_to_dynamic(typed))
            })
            .collect::<XeResult<Vec<_>>>()?;
        Ok((
            TypedExpressionKind::DynamicCall {
                callee: Box::new(self.coerce_to_dynamic(callee)),
                args,
            },
            XeType::Unknown,
        ))
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
}

/// Definite-assignment state after two paths join. `None` means the path cannot be
/// reached (it returned, broke or continued), so it imposes no constraint.
fn meet(a: Option<HashSet<String>>, b: Option<HashSet<String>>) -> Option<HashSet<String>> {
    match (a, b) {
        (None, other) | (other, None) => other,
        (Some(a), Some(b)) => Some(a.intersection(&b).cloned().collect()),
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

/// Names assigned by top-level code (including inside blocks), excluding loop variables
/// inside their own loop. Mirrors the linker's notion of module-level variables.
fn collect_top_level_targets(statement: &Statement, targets: &mut Vec<String>) {
    match &statement.kind {
        StatementKind::Assignment { name, .. } => targets.push(name.clone()),
        StatementKind::Try {
            body,
            catch_variable,
            handler,
        } => {
            body.iter().for_each(|s| collect_top_level_targets(s, targets));
            targets.extend(catch_variable.iter().cloned());
            handler.iter().for_each(|s| collect_top_level_targets(s, targets));
        }
        StatementKind::If {
            then_block,
            else_block,
            ..
        } => {
            then_block.iter().for_each(|s| collect_top_level_targets(s, targets));
            if let Some(else_block) = else_block {
                else_block.iter().for_each(|s| collect_top_level_targets(s, targets));
            }
        }
        StatementKind::For { variable, body, .. } => {
            let mut inner = Vec::new();
            body.iter().for_each(|s| collect_top_level_targets(s, &mut inner));
            targets.extend(inner.into_iter().filter(|name| name != variable));
        }
        StatementKind::While { body, .. } | StatementKind::Repeat { body, .. } => {
            body.iter().for_each(|s| collect_top_level_targets(s, targets));
        }
        _ => {}
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
            StatementKind::Try { body, handler, .. } => {
                collect_global_names(body, names);
                collect_global_names(handler, names);
            }
            StatementKind::While { body, .. }
            | StatementKind::Repeat { body, .. }
            | StatementKind::For { body, .. } => collect_global_names(body, names),
            _ => {}
        }
    }
}
