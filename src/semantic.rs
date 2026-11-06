use std::collections::{HashMap, HashSet};

use crate::ast::*;
use crate::builtins::Builtin;
use crate::error::{Span, XeError, XeErrorKind, XeResult};

/// Analyzes a linked program, then tries to give function parameters and return
/// values native types (see [`optimize`]). The result is always equivalent to the
/// conservative analysis; typing is only made more precise when that is provably safe.
pub fn analyze_program(program: &Program) -> XeResult<TypedProgram> {
    let mut analyzer = SemanticAnalyzer::new(Seeds::default());
    let base = analyzer.analyze(program)?;
    Ok(optimize(program, base, analyzer.report))
}

/// Type guesses for function parameters and return values.
#[derive(Clone, Default, PartialEq)]
struct Seeds {
    params: HashMap<String, Vec<XeType>>,
    returns: HashMap<String, XeType>,
}

/// Facts recorded during one analysis, used to compute and verify seeds.
#[derive(Clone, Default)]
struct Report {
    /// Argument types at each direct call of a user function.
    call_sites: HashMap<String, Vec<Vec<XeType>>>,
    /// Functions used as values; their parameters must stay dynamic.
    function_values: HashSet<String>,
    returns: HashMap<String, XeType>,
    /// The return type a function would have if its dynamic return values were native.
    optimistic_returns: HashMap<String, XeType>,
    /// Declared type of every variable ("function::name", "::name" for globals).
    var_types: HashMap<String, XeType>,
    /// Variables that received a runtime-checked conversion from a dynamic value.
    checked_vars: HashSet<String>,
}

/// Repeatedly re-analyzes the program with native parameter and return types inferred
/// from the previous round. A round is accepted only if every guess is confirmed:
/// call sites pass exactly the seeded types, functions return exactly the seeded types,
/// and no variable became native while also receiving runtime-checked dynamic values.
/// Otherwise the conservative analysis is used, so behavior never changes.
fn optimize(program: &Program, base: TypedProgram, base_report: Report) -> TypedProgram {
    let mut demoted_params: HashSet<(String, usize)> = HashSet::new();
    let mut failed_returns: HashSet<String> = HashSet::new();
    let mut report = base_report.clone();
    let mut best = base;
    let mut tried: Vec<Seeds> = Vec::new();

    // Each round may reveal more (e.g. native parameters make a recursive function's
    // return type native), so keep refining while the guesses change.
    for _ in 0..8 {
        let seeds = seeds_from(&report, &demoted_params, &failed_returns);
        if (seeds.params.is_empty() && seeds.returns.is_empty()) || tried.contains(&seeds) {
            break;
        }
        tried.push(seeds.clone());

        let mut analyzer = SemanticAnalyzer::new(seeds.clone());
        let Ok(typed) = analyzer.analyze(program) else {
            break;
        };
        let round = analyzer.report;

        let mut consistent = true;
        for (function, seeded) in &seeds.returns {
            if round.returns.get(function) != Some(seeded) {
                failed_returns.insert(function.clone());
                consistent = false;
            }
        }
        for (function, params) in &seeds.params {
            for (i, seeded) in params.iter().enumerate() {
                if *seeded == XeType::Unknown {
                    continue;
                }
                let mismatch = round
                    .call_sites
                    .get(function)
                    .is_some_and(|sites| sites.iter().any(|site| site.get(i) != Some(seeded)));
                if mismatch {
                    demoted_params.insert((function.clone(), i));
                    consistent = false;
                }
            }
        }
        for var in &round.checked_vars {
            if round.var_types.get(var) != base_report.var_types.get(var) {
                let function = var.split("::").next().unwrap_or_default();
                if function.is_empty() {
                    return best;
                }
                // Back off every guess about the function this variable lives in.
                if let Some(params) = seeds.params.get(function) {
                    for i in 0..params.len() {
                        demoted_params.insert((function.to_string(), i));
                    }
                }
                failed_returns.insert(function.to_string());
                consistent = false;
            }
        }

        if consistent {
            best = typed;
        }
        report = round;
    }

    best
}

fn seeds_from(
    report: &Report,
    demoted_params: &HashSet<(String, usize)>,
    failed_returns: &HashSet<String>,
) -> Seeds {
    let mut seeds = Seeds::default();

    for (function, sites) in &report.call_sites {
        if report.function_values.contains(function) {
            continue;
        }
        let Some(first) = sites.first() else {
            continue;
        };
        let params: Vec<XeType> = (0..first.len())
            .map(|i| {
                let ty = &first[i];
                let agreed = sites.iter().all(|site| site.get(i) == Some(ty));
                if agreed && ty.is_scalar() && !demoted_params.contains(&(function.clone(), i)) {
                    ty.clone()
                } else {
                    XeType::Unknown
                }
            })
            .collect();
        if params.iter().any(|ty| *ty != XeType::Unknown) {
            seeds.params.insert(function.clone(), params);
        }
    }

    for (function, inferred) in &report.returns {
        if failed_returns.contains(function) {
            continue;
        }
        if inferred.is_scalar() {
            seeds.returns.insert(function.clone(), inferred.clone());
        } else if let Some(optimistic) = report.optimistic_returns.get(function) {
            seeds.returns.insert(function.clone(), optimistic.clone());
        }
    }

    seeds
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
    param_names: Vec<String>,
    defaults: Vec<TypedExpression>,
    return_type: XeType,
    is_struct: bool,
}

enum ArgSource<'a> {
    Receiver,
    Expression(&'a Expression),
    Default,
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
    name: String,
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
    struct_methods: HashMap<String, HashMap<String, String>>,
    method_names: HashMap<String, Vec<String>>,
    loop_depth: usize,
    function: Option<FunctionState>,
    lambdas: Vec<LambdaFrame>,
    seeds: Seeds,
    report: Report,
}

impl SemanticAnalyzer {
    fn new(seeds: Seeds) -> Self {
        Self {
            globals: HashMap::new(),
            global_order: Vec::new(),
            scopes: Vec::new(),
            functions: HashMap::new(),
            structs: HashMap::new(),
            struct_methods: HashMap::new(),
            method_names: HashMap::new(),
            loop_depth: 0,
            function: None,
            lambdas: Vec::new(),
            seeds,
            report: Report::default(),
        }
    }

    fn analyze(&mut self, program: &Program) -> XeResult<TypedProgram> {
        // Pass 1: Collect function signatures, structs and module-level variables
        for stmt in &program.statements {
            match &stmt.kind {
                StatementKind::FunctionDef { name, params, body, .. } => {
                    self.check_new_definition(name, &stmt.span)?;
                    self.register_function(name, params, body, None, &stmt.span);
                }
                StatementKind::StructDef {
                    name,
                    fields,
                    methods,
                    ..
                } => {
                    self.check_new_definition(name, &stmt.span)?;
                    let mut member_names = fields.clone();
                    self.structs.insert(name.clone(), fields.clone());
                    self.functions.insert(name.clone(), FunctionInfo {
                        params: vec![XeType::Unknown; fields.len()],
                        param_names: fields.clone(),
                        defaults: Vec::new(),
                        return_type: XeType::Struct(name.clone()),
                        is_struct: true,
                    });
                    let mut table = HashMap::new();
                    for method in methods {
                        let StatementKind::FunctionDef {
                            name: method_name,
                            params,
                            body,
                            ..
                        } = &method.kind
                        else {
                            unreachable!("struct methods are function definitions");
                        };
                        if params.is_empty() {
                            return Err(XeError::new(
                                XeErrorKind::InvalidOperation(format!(
                                    "method '{method_name}' must take the struct as its first parameter, like '{method_name}(self)'"
                                )),
                                Some(method.span.clone()),
                            ));
                        }
                        member_names.push(method_name.clone());
                        let symbol = method_symbol(name, method_name);
                        self.register_function(
                            &symbol,
                            params,
                            body,
                            Some(XeType::Struct(name.clone())),
                            &method.span,
                        );
                        self.method_names
                            .entry(method_name.clone())
                            .or_default()
                            .push(symbol.clone());
                        table.insert(method_name.clone(), symbol);
                    }
                    check_unique_names(&member_names, &stmt.span)?;
                    self.struct_methods.insert(name.clone(), table);
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

        for stmt in &program.statements {
            match &stmt.kind {
                StatementKind::FunctionDef { name, defaults, .. } => {
                    self.analyze_defaults(name, defaults)?;
                }
                StatementKind::StructDef {
                    name,
                    defaults,
                    methods,
                    ..
                } => {
                    self.analyze_defaults(name, defaults)?;
                    for method in methods {
                        if let StatementKind::FunctionDef {
                            name: method_name,
                            defaults,
                            ..
                        } = &method.kind
                        {
                            self.analyze_defaults(&method_symbol(name, method_name), defaults)?;
                        }
                    }
                }
                _ => {}
            }
        }

        // Pass 2: Full semantic analysis and IR production
        let mut typed_statements = Vec::new();
        for stmt in &program.statements {
            if let StatementKind::StructDef {
                name,
                fields,
                methods,
                ..
            } = &stmt.kind
            {
                let mut method_table = Vec::new();
                let mut method_defs = Vec::new();
                for method in methods {
                    let StatementKind::FunctionDef {
                        name: method_name,
                        params,
                        body,
                        ..
                    } = &method.kind
                    else {
                        unreachable!("struct methods are function definitions");
                    };
                    let symbol = method_symbol(name, method_name);
                    let kind = self.analyze_function(&symbol, params, body, &method.span)?;
                    method_table.push((method_name.clone(), symbol));
                    method_defs.push(TypedStatement {
                        kind,
                        span: method.span.clone(),
                    });
                }
                typed_statements.push(TypedStatement {
                    kind: TypedStatementKind::StructDef {
                        name: name.clone(),
                        fields: fields.clone(),
                        defaults: self.functions[name].defaults.clone(),
                        methods: method_table,
                    },
                    span: stmt.span.clone(),
                });
                typed_statements.extend(method_defs);
                continue;
            }
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

    fn register_function(
        &mut self,
        name: &str,
        params: &[String],
        body: &[Statement],
        receiver: Option<XeType>,
        span: &Span,
    ) {
        let mut param_types = self
            .seeds
            .params
            .get(name)
            .cloned()
            .unwrap_or_else(|| vec![XeType::Unknown; params.len()]);
        if let Some(receiver) = receiver {
            param_types[0] = receiver;
        }
        let return_type = self
            .seeds
            .returns
            .get(name)
            .cloned()
            .unwrap_or(XeType::Unknown);
        self.functions.insert(name.to_string(), FunctionInfo {
            params: param_types,
            param_names: params.to_vec(),
            defaults: Vec::new(),
            return_type,
            is_struct: false,
        });

        let mut declared_globals = Vec::new();
        collect_global_names(body, &mut declared_globals);
        for global in declared_globals {
            self.declare_global_placeholder(&global, span);
        }
    }

    fn analyze_defaults(&mut self, name: &str, defaults: &[Expression]) -> XeResult<()> {
        let mut typed = Vec::with_capacity(defaults.len());
        for default in defaults {
            let value = self.analyze_expression(default)?;
            typed.push(if value.ty.is_scalar() {
                value
            } else {
                self.coerce_to_dynamic(value)
            });
        }
        if let Some(info) = self.functions.get_mut(name) {
            info.defaults = typed;
        }
        Ok(())
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
                let body = self.analyze_loop_body(body, &[], &stmt.span)?;
                TypedStatementKind::While { condition, body }
            }
            StatementKind::Repeat { count, body } => {
                let count = self.analyze_expression(count)?;
                let count = self
                    .coerce(count, &XeType::Number)
                    .map_err(|got| type_mismatch("number", got, &stmt.span))?;
                let body = self.analyze_loop_body(body, &[], &stmt.span)?;
                TypedStatementKind::Repeat { count, body }
            }
            StatementKind::For {
                variables,
                iterable,
                body,
            } => {
                let (typed_iter, variables) = self.loop_variables(variables, iterable, &stmt.span)?;
                for (name, ty) in &variables {
                    self.record_var_type(name, ty);
                }
                let body = self.analyze_loop_body(body, &variables, &stmt.span)?;

                TypedStatementKind::For {
                    variables,
                    iterable: typed_iter,
                    body,
                }
            }
            StatementKind::FunctionDef { name, params, body, .. } => {
                self.analyze_function(name, params, body, &stmt.span)?
            }
            StatementKind::StructDef { .. } => {
                return Err(XeError::new(XeErrorKind::NestedDefinition, Some(stmt.span.clone())));
            }
            StatementKind::Unpack { targets, value } => {
                let typed_value = self.analyze_expression(value)?;
                let typed_value = self.coerce_to_dynamic(typed_value);
                let mut assignments = Vec::with_capacity(targets.len());
                for (i, target) in targets.iter().enumerate() {
                    let item = TypedExpression {
                        kind: TypedExpressionKind::UnpackItem(i),
                        ty: XeType::Unknown,
                        span: target.span.clone(),
                    };
                    let kind = match &target.kind {
                        ExpressionKind::Identifier(name) => TypedStatementKind::Assignment {
                            name: name.clone(),
                            value: self.assign_variable(name, item, &stmt.span)?,
                        },
                        ExpressionKind::Index { object, index } => {
                            self.index_assignment(object, index, item, None, &stmt.span)?
                        }
                        ExpressionKind::FieldAccess { object, field } => {
                            self.field_assignment(object, field, item, None, &stmt.span)?
                        }
                        _ => {
                            return Err(XeError::new(
                                XeErrorKind::ExpectedToken("valid assignment target".to_string()),
                                Some(target.span.clone()),
                            ));
                        }
                    };
                    assignments.push(TypedStatement {
                        kind,
                        span: stmt.span.clone(),
                    });
                }
                TypedStatementKind::Unpack {
                    value: typed_value,
                    count: targets.len(),
                    assignments,
                }
            }
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
                self.index_assignment(object, index, typed_value, *op, &stmt.span)?
            }
            StatementKind::FieldAssignment {
                object,
                field,
                value,
                op,
            } => {
                let typed_value = self.analyze_expression(value)?;
                self.field_assignment(object, field, typed_value, *op, &stmt.span)?
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

    fn index_assignment(
        &mut self,
        object: &Expression,
        index: &Expression,
        typed_value: TypedExpression,
        op: Option<BinaryOperator>,
        span: &Span,
    ) -> XeResult<TypedStatementKind> {
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
                        "cannot assign to an element of a {ty} value"
                    )),
                    Some(span.clone()),
                ));
            }
        }
        Ok(TypedStatementKind::IndexAssignment {
            object: typed_object,
            index: self.coerce_to_dynamic(typed_index),
            value: self.coerce_to_dynamic(typed_value),
            op,
        })
    }

    fn field_assignment(
        &mut self,
        object: &Expression,
        field: &str,
        typed_value: TypedExpression,
        op: Option<BinaryOperator>,
        span: &Span,
    ) -> XeResult<TypedStatementKind> {
        let typed_object = self.analyze_expression(object)?;
        self.check_field_access(&typed_object, field, span, true)?;
        Ok(TypedStatementKind::FieldAssignment {
            object: typed_object,
            field: field.to_string(),
            value: self.coerce_to_dynamic(typed_value),
            op,
        })
    }

    fn loop_variables(
        &mut self,
        variables: &[String],
        iterable: &Expression,
        span: &Span,
    ) -> XeResult<(TypedExpression, Vec<(String, XeType)>)> {
        check_unique_names(variables, span)?;
        let typed_iter = self.analyze_expression(iterable)?;
        let is_range = matches!(
            typed_iter.kind,
            TypedExpressionKind::BuiltinCall {
                builtin: Builtin::Range,
                ..
            }
        );
        let item_type = match &typed_iter.ty {
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
        if variables.len() > 1 && item_type != XeType::Unknown {
            return Err(XeError::new(
                XeErrorKind::InvalidOperation(format!(
                    "cannot unpack the items of a {} into {} variables",
                    if is_range { "range".to_string() } else { typed_iter.ty.name() },
                    variables.len()
                )),
                Some(iterable.span.clone()),
            ));
        }
        let typed_variables = if variables.len() == 1 {
            vec![(variables[0].clone(), item_type)]
        } else {
            variables
                .iter()
                .map(|name| (name.clone(), XeType::Unknown))
                .collect()
        };
        Ok((typed_iter, typed_variables))
    }

    fn variable_scope(variables: &[(String, XeType)], span: &Span) -> Scope {
        Scope {
            kind: ScopeKind::Loop,
            vars: variables
                .iter()
                .map(|(name, ty)| {
                    (name.clone(), SymbolInfo {
                        ty: ty.clone(),
                        defined_at: span.clone(),
                        declared: true,
                    })
                })
                .collect(),
        }
    }

    /// Analyzes a loop body. Assignments inside it are not definite after the loop,
    /// since the body may run zero times.
    fn analyze_loop_body(
        &mut self,
        body: &[Statement],
        variables: &[(String, XeType)],
        span: &Span,
    ) -> XeResult<Vec<TypedStatement>> {
        let before = self.flow();
        self.loop_depth += 1;
        let has_variables = !variables.is_empty();
        if has_variables {
            self.scopes.push(Self::variable_scope(variables, span));
        }
        let typed = self.analyze_statements(body);
        if has_variables {
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
            self.report
                .var_types
                .insert(format!("{name}::{param}"), ty.clone());
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
            name: name.to_string(),
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
        if let Some(optimistic) = optimistic_return_type(&state.returns, always_returns) {
            self.report
                .optimistic_returns
                .insert(name.to_string(), optimistic);
        }
        self.report.returns.insert(name.to_string(), return_type.clone());
        if let Some(info) = self.functions.get_mut(name) {
            info.return_type = return_type.clone();
        }

        Ok(TypedStatementKind::FunctionDef {
            name: name.to_string(),
            params: params.iter().cloned().zip(param_types).collect(),
            defaults: self.functions[name].defaults.clone(),
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
        if let Some(function) = &mut self.function
            && let Some(assigned) = &mut function.assigned {
                assigned.insert(name.to_string());
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

    fn var_key(&self, name: &str) -> String {
        match &self.function {
            Some(function) => format!("{}::{}", function.name, name),
            None => format!("::{name}"),
        }
    }

    fn record_var_type(&mut self, name: &str, ty: &XeType) {
        let key = self.var_key(name);
        self.report.var_types.insert(key, ty.clone());
    }

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
                let checked = value.ty.is_dynamic() && info.ty.is_scalar();
                let coerced = self.coerce(value, &info.ty).map_err(|got| {
                    XeError::new(
                        XeErrorKind::TypeMismatch {
                            expected: format!("{} (defined at line {})", info.ty, info.defined_at.line),
                            got: got.name(),
                        },
                        Some(span.clone()),
                    )
                })?;
                if checked {
                    let key = match location {
                        VarLocation::Global => format!("::{name}"),
                        VarLocation::Local(_) => self.var_key(name),
                    };
                    self.report.checked_vars.insert(key);
                }
                if let VarLocation::Local(index) = location
                    && self.scopes[index].kind == ScopeKind::FunctionLocals {
                        self.mark_assigned(name);
                    }
                Ok(coerced)
            }
            Some((VarLocation::Global, _)) => {
                // First assignment of a known module-level variable.
                let global = self.globals.get_mut(name).expect("found above");
                global.ty = value.ty.clone();
                global.declared = true;
                global.defined_at = span.clone();
                self.report
                    .var_types
                    .insert(format!("::{name}"), value.ty.clone());
                Ok(value)
            }
            Some((VarLocation::Local(_), _)) => unreachable!("locals are always declared"),
            None => {
                let info = SymbolInfo {
                    ty: value.ty.clone(),
                    defined_at: span.clone(),
                    declared: true,
                };
                self.record_var_type(name, &value.ty);
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
                    self.report.function_values.insert(name.clone());
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
                    self.dynamic_call(callee, args, &format!("'{name}'"))?
                } else {
                    self.named_call(name, None, args, &span)?
                }
            }

            ExpressionKind::MethodCall {
                object,
                method,
                function,
                args,
            } => {
                let receiver = self.analyze_expression(object)?;
                let function_name = function.clone().unwrap_or_else(|| method.clone());
                let struct_method = match &receiver.ty {
                    XeType::Struct(name) => self
                        .struct_methods
                        .get(name)
                        .and_then(|methods| methods.get(method))
                        .cloned(),
                    _ => None,
                };
                if let Some(symbol) = struct_method {
                    self.named_call(&symbol, Some(receiver), args, &span)?
                } else if receiver.ty == XeType::Unknown && self.method_names.contains_key(method) {
                    self.dynamic_method_call(receiver, method, &function_name, args)?
                } else if self.functions.contains_key(&function_name) || Builtin::is_builtin(&function_name) {
                    // `object.method(args)` is `method(object, args)`.
                    self.named_call(&function_name, Some(receiver), args, &span)?
                } else {
                    self.check_field_access(&receiver, method, &span, false)?;
                    let callee = TypedExpression {
                        kind: TypedExpressionKind::FieldAccess {
                            object: Box::new(receiver),
                            field: method.clone(),
                        },
                        ty: XeType::Unknown,
                        span: span.clone(),
                    };
                    self.dynamic_call(callee, args, &format!("'{method}'"))?
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

            ExpressionKind::Conditional {
                condition,
                then_value,
                else_value,
            } => {
                let condition = self.analyze_condition(condition)?;
                let then_value = self.analyze_expression(then_value)?;
                let else_value = self.analyze_expression(else_value)?;
                let (then_value, else_value, ty) = if then_value.ty == else_value.ty {
                    let ty = then_value.ty.clone();
                    (then_value, else_value, ty)
                } else {
                    (
                        self.coerce_to_dynamic(then_value),
                        self.coerce_to_dynamic(else_value),
                        XeType::Unknown,
                    )
                };
                (
                    TypedExpressionKind::Conditional {
                        condition: Box::new(condition),
                        then_value: Box::new(then_value),
                        else_value: Box::new(else_value),
                    },
                    ty,
                )
            }

            ExpressionKind::Comprehension { kind, clauses } => {
                let base = self.scopes.len();
                let result = self.comprehension(kind, clauses, &span);
                self.scopes.truncate(base);
                result?
            }

            ExpressionKind::KeywordArg { name, .. } => {
                return Err(XeError::new(
                    XeErrorKind::Syntax(format!(
                        "keyword argument '{name}' can only be used in a function call"
                    )),
                    Some(span),
                ));
            }
        };

        let typed = TypedExpression { kind, ty, span };
        if typed.ty == XeType::Void {
            // A call to a function without a return value evaluates to `none`.
            return Ok(self.wrap_to_unknown(typed));
        }
        Ok(typed)
    }

    fn comprehension(
        &mut self,
        kind: &ComprehensionKind,
        clauses: &[ComprehensionClause],
        span: &Span,
    ) -> XeResult<(TypedExpressionKind, XeType)> {
        let mut typed_clauses = Vec::with_capacity(clauses.len());
        for clause in clauses {
            typed_clauses.push(match clause {
                ComprehensionClause::For { variables, iterable } => {
                    let (iterable, variables) = self.loop_variables(variables, iterable, span)?;
                    self.scopes.push(Self::variable_scope(&variables, span));
                    TypedComprehensionClause::For { variables, iterable }
                }
                ComprehensionClause::If(condition) => {
                    TypedComprehensionClause::If(self.analyze_condition(condition)?)
                }
            });
        }
        let (kind, ty) = match kind {
            ComprehensionKind::List(element) => {
                let element = self.analyze_expression(element)?;
                (
                    TypedComprehensionKind::List(Box::new(self.coerce_to_dynamic(element))),
                    XeType::List,
                )
            }
            ComprehensionKind::Map(key, value) => {
                let key = self.analyze_expression(key)?;
                let value = self.analyze_expression(value)?;
                (
                    TypedComprehensionKind::Map(
                        Box::new(self.coerce_to_dynamic(key)),
                        Box::new(self.coerce_to_dynamic(value)),
                    ),
                    XeType::Map,
                )
            }
        };
        Ok((
            TypedExpressionKind::Comprehension {
                kind,
                clauses: typed_clauses,
            },
            ty,
        ))
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
                        XeErrorKind::UndefinedVariable(format!("{struct_name}.{field}")),
                        Some(span.clone()),
                    ))
                }
            }
            XeType::Map | XeType::Unknown => Ok(()),
            ty => {
                let message = if is_assignment {
                    format!("cannot assign field '{field}' on a {ty} value")
                } else {
                    format!("cannot access field '{field}' on a {ty} value")
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
        receiver: Option<TypedExpression>,
        args: &[Expression],
        span: &Span,
    ) -> XeResult<(TypedExpressionKind, XeType)> {
        let (positional, keywords) = split_arguments(args);

        if let Some(info) = self.functions.get(name) {
            let param_types = info.params.clone();
            let param_names = info.param_names.clone();
            let defaults = info.defaults.clone();
            let return_type = info.return_type.clone();
            let is_struct = info.is_struct;
            let required = param_types.len() - defaults.len();

            let mut sources: Vec<Option<ArgSource>> = (0..param_types.len()).map(|_| None).collect();
            let mut order = Vec::new();
            let mut given = 0;
            if receiver.is_some() {
                given += 1;
            }
            given += positional.len();
            if given > param_types.len() || (keywords.is_empty() && given < required) {
                return Err(wrong_argument_count(name, required, param_types.len(), given, span));
            }
            let mut position = 0;
            if receiver.is_some() {
                sources[0] = Some(ArgSource::Receiver);
                order.push(0);
                position = 1;
            }
            for arg in &positional {
                sources[position] = Some(ArgSource::Expression(arg));
                order.push(position);
                position += 1;
            }
            for (keyword, value, keyword_span) in &keywords {
                let Some(index) = param_names.iter().position(|p| p == *keyword) else {
                    return Err(XeError::new(
                        XeErrorKind::InvalidOperation(format!(
                            "{} '{name}' has no {} named '{keyword}'",
                            if is_struct { "struct" } else { "function" },
                            if is_struct { "field" } else { "parameter" },
                        )),
                        Some((*keyword_span).clone()),
                    ));
                };
                if sources[index].is_some() {
                    return Err(XeError::new(
                        XeErrorKind::InvalidOperation(format!(
                            "argument '{keyword}' of '{name}' is given more than once"
                        )),
                        Some((*keyword_span).clone()),
                    ));
                }
                sources[index] = Some(ArgSource::Expression(value));
                order.push(index);
            }
            for (index, source) in sources.iter_mut().enumerate() {
                if source.is_none() {
                    if index < required {
                        return Err(XeError::new(
                            XeErrorKind::InvalidOperation(format!(
                                "missing argument '{}' in call to '{name}'",
                                param_names[index]
                            )),
                            Some(span.clone()),
                        ));
                    }
                    *source = Some(ArgSource::Default);
                }
            }

            let mut receiver = receiver;
            let mut site_types = vec![XeType::Unknown; param_types.len()];
            let mut typed_args: Vec<Option<TypedExpression>> = (0..param_types.len()).map(|_| None).collect();
            let evaluation: Vec<usize> = order
                .iter()
                .copied()
                .chain((0..param_types.len()).filter(|i| !order.contains(i)))
                .collect();
            for index in evaluation {
                let (typed, arg_span) = match sources[index].as_ref().expect("bound above") {
                    ArgSource::Receiver => {
                        let typed = receiver.take().expect("receiver is bound once");
                        let arg_span = typed.span.clone();
                        (typed, arg_span)
                    }
                    ArgSource::Expression(arg) => (self.analyze_expression(arg)?, arg.span.clone()),
                    ArgSource::Default => {
                        let default = &defaults[index - required];
                        (
                            TypedExpression {
                                kind: TypedExpressionKind::DefaultArg {
                                    function: name.to_string(),
                                    index,
                                },
                                ty: default.ty.clone(),
                                span: span.clone(),
                            },
                            span.clone(),
                        )
                    }
                };
                site_types[index] = typed.ty.clone();
                let param_ty = &param_types[index];
                let typed = self
                    .coerce(typed, param_ty)
                    .map_err(|got| type_mismatch(&param_ty.name(), got, &arg_span))?;
                typed_args[index] = Some(typed);
            }
            if !is_struct {
                self.report
                    .call_sites
                    .entry(name.to_string())
                    .or_default()
                    .push(site_types);
            }
            let natural = order.windows(2).all(|pair| pair[0] < pair[1]);
            return Ok((
                TypedExpressionKind::FunctionCall {
                    name: name.to_string(),
                    args: typed_args.into_iter().map(|arg| arg.expect("every argument is bound")).collect(),
                    order: if natural { Vec::new() } else { order },
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
        if let Some((keyword, _, keyword_span)) = keywords.first() {
            return Err(XeError::new(
                XeErrorKind::InvalidOperation(format!(
                    "built-in function '{name}' does not accept keyword arguments (got '{keyword}')"
                )),
                Some((*keyword_span).clone()),
            ));
        }
        let receiver_count = usize::from(receiver.is_some());
        let count = receiver_count + positional.len();
        let signature = builtin.signature();
        if !signature.accepts(count) {
            return Err(XeError::new(
                XeErrorKind::InvalidOperation(format!(
                    "function '{}' expects {} arguments, got {}",
                    name,
                    signature.describe_arity(),
                    count
                )),
                Some(span.clone()),
            ));
        }

        let mut original_types = Vec::new();
        let mut typed_args = Vec::new();
        let mut receiver = receiver;
        for i in 0..count {
            let (typed, arg_span) = if i < receiver_count {
                let typed = receiver.take().expect("receiver is used once");
                let arg_span = typed.span.clone();
                (typed, arg_span)
            } else {
                let arg = positional[i - receiver_count];
                (self.analyze_expression(arg)?, arg.span.clone())
            };
            original_types.push(typed.ty.clone());
            let expected = signature.param_type(i);
            if !typed.ty.is_compatible(&expected) {
                return Err(type_mismatch(&expected.name(), typed.ty, &arg_span));
            }
            let typed = if expected.is_dynamic() {
                self.coerce_to_dynamic(typed)
            } else {
                self.coerce(typed, &expected).unwrap_or_else(|_| unreachable!())
            };
            typed_args.push(typed);
        }

        let return_type = match builtin {
            Builtin::Convert => {
                let target = positional[1 - receiver_count];
                match &target.kind {
                    ExpressionKind::String(target_name) => match target_name.as_str() {
                        "number" => XeType::Number,
                        "text" => XeType::Text,
                        "boolean" => XeType::Boolean,
                        other => {
                            return Err(XeError::new(
                                XeErrorKind::InvalidOperation(format!(
                                    "convert() target must be \"number\", \"text\" or \"boolean\", got \"{other}\""
                                )),
                                Some(target.span.clone()),
                            ));
                        }
                    },
                    _ => XeType::Unknown,
                }
            }
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

    fn dynamic_arguments(
        &mut self,
        args: &[Expression],
    ) -> XeResult<DynamicArguments> {
        let (positional, keywords) = split_arguments(args);
        let mut typed_positional = Vec::with_capacity(positional.len());
        for arg in positional {
            let typed = self.analyze_expression(arg)?;
            typed_positional.push(self.coerce_to_dynamic(typed));
        }
        let mut typed_keywords = Vec::with_capacity(keywords.len());
        for (name, value, _) in keywords {
            let typed = self.analyze_expression(value)?;
            typed_keywords.push((name.clone(), self.coerce_to_dynamic(typed)));
        }
        Ok((typed_positional, typed_keywords))
    }

    fn dynamic_method_call(
        &mut self,
        receiver: TypedExpression,
        method: &str,
        function_name: &str,
        args: &[Expression],
    ) -> XeResult<(TypedExpressionKind, XeType)> {
        for symbol in self.method_names.get(method).cloned().unwrap_or_default() {
            self.report.function_values.insert(symbol);
        }
        let fallback = if self.functions.contains_key(function_name) {
            self.report.function_values.insert(function_name.to_string());
            Some(TypedExpressionKind::FunctionRef(function_name.to_string()))
        } else {
            Builtin::from_name(function_name).map(TypedExpressionKind::BuiltinRef)
        };
        let fallback = fallback.map(|kind| {
            Box::new(TypedExpression {
                kind,
                ty: XeType::Function,
                span: receiver.span.clone(),
            })
        });
        let (args, keywords) = self.dynamic_arguments(args)?;
        Ok((
            TypedExpressionKind::MethodCall {
                object: Box::new(receiver),
                method: method.to_string(),
                args,
                keywords,
                fallback,
            },
            XeType::Unknown,
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
        let (args, keywords) = self.dynamic_arguments(args)?;
        Ok((
            TypedExpressionKind::DynamicCall {
                callee: Box::new(self.coerce_to_dynamic(callee)),
                args,
                keywords,
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

type KeywordArgument<'a> = (&'a String, &'a Expression, &'a Span);
type DynamicArguments = (Vec<TypedExpression>, Vec<(String, TypedExpression)>);

fn split_arguments(args: &[Expression]) -> (Vec<&Expression>, Vec<KeywordArgument<'_>>) {
    let mut positional = Vec::new();
    let mut keywords = Vec::new();
    for arg in args {
        match &arg.kind {
            ExpressionKind::KeywordArg { name, value } => keywords.push((name, &**value, &arg.span)),
            _ => positional.push(arg),
        }
    }
    (positional, keywords)
}

fn wrong_argument_count(name: &str, required: usize, total: usize, got: usize, span: &Span) -> XeError {
    if required == total {
        return XeError::new(
            XeErrorKind::WrongArgumentCount {
                name: name.to_string(),
                expected: total,
                got,
            },
            Some(span.clone()),
        );
    }
    XeError::new(
        XeErrorKind::InvalidOperation(format!(
            "function '{name}' expects {required} to {total} arguments, got {got}"
        )),
        Some(span.clone()),
    )
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

/// The native type a function would return if its dynamic return values (typically
/// recursive calls) turn out to have the same type as its native ones.
fn optimistic_return_type(info: &ReturnInfo, always_returns: bool) -> Option<XeType> {
    if info.has_bare_return || !always_returns {
        return None;
    }
    let mut native = info.value_types.iter().filter(|ty| **ty != XeType::Unknown);
    let first = native.next()?;
    (first.is_scalar() && native.all(|ty| ty == first)).then(|| first.clone())
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
        StatementKind::Unpack { targets: items, .. } => {
            for item in items {
                if let ExpressionKind::Identifier(name) = &item.kind {
                    targets.push(name.clone());
                }
            }
        }
        StatementKind::Try {
            body,
            catch_variable,
            handler,
        } => {
            for s in body {
                collect_top_level_targets(s, targets);
            }
            targets.extend(catch_variable.iter().cloned());
            for s in handler {
                collect_top_level_targets(s, targets);
            }
        }
        StatementKind::If {
            then_block,
            else_block,
            ..
        } => {
            for s in then_block {
                collect_top_level_targets(s, targets);
            }
            if let Some(else_block) = else_block {
                for s in else_block {
                    collect_top_level_targets(s, targets);
                }
            }
        }
        StatementKind::For { variables, body, .. } => {
            let mut inner = Vec::new();
            for s in body {
                collect_top_level_targets(s, &mut inner);
            }
            targets.extend(inner.into_iter().filter(|name| !variables.contains(name)));
        }
        StatementKind::While { body, .. } | StatementKind::Repeat { body, .. } => {
            for s in body {
                collect_top_level_targets(s, targets);
            }
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
