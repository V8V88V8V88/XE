use std::collections::HashMap;

use crate::ast::*;
use crate::builtins::Builtin;
use crate::error::Span;

/// The XE runtime, emitted at the top of every generated program.
const PRELUDE: &str = include_str!("runtime/prelude.rs");

pub struct CodeGenerator {
    output: String,
    indent_level: usize,
    /// Local variables in scope and their declared types.
    scopes: Vec<HashMap<String, XeType>>,
    /// Module-level variables and their storage types.
    globals: HashMap<String, XeType>,
    function_params: HashMap<String, Vec<XeType>>,
    function_returns: HashMap<String, XeType>,
    /// Source text of every module, for runtime error messages.
    sources: HashMap<String, String>,
    /// (file, line, source line) of statements; index 0 means "no location".
    locations: Vec<(String, usize, String)>,
    location_ids: HashMap<(String, usize), usize>,
    current_location: usize,
    /// Return type of the function being generated (`Void` for top-level code).
    return_type: XeType,
    /// Number of `try` bodies (which run in closures) around the current statement.
    try_depth: usize,
    /// `try_depth` at each enclosing loop, innermost last.
    loop_try_depths: Vec<usize>,
    builtin_values: Vec<Builtin>,
    temp_counter: usize,
}

impl CodeGenerator {
    pub fn new(sources: HashMap<String, String>) -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            scopes: Vec::new(),
            globals: HashMap::new(),
            function_params: HashMap::new(),
            function_returns: HashMap::new(),
            sources,
            locations: vec![(String::new(), 0, String::new())],
            location_ids: HashMap::new(),
            current_location: 0,
            return_type: XeType::Void,
            try_depth: 0,
            loop_try_depths: Vec::new(),
            builtin_values: Vec::new(),
            temp_counter: 0,
        }
    }

    pub fn generate(&mut self, program: &TypedProgram) -> String {
        self.emit(PRELUDE);
        self.emit("\n// ---------------------------------------------------------------------------\n");
        self.emit("// Program\n");
        self.emit("// ---------------------------------------------------------------------------\n\n");

        self.globals = program.globals.iter().cloned().collect();
        for stmt in &program.statements {
            match &stmt.kind {
                TypedStatementKind::FunctionDef {
                    name,
                    params,
                    return_type,
                    ..
                } => {
                    self.function_params
                        .insert(name.clone(), params.iter().map(|(_, ty)| ty.clone()).collect());
                    self.function_returns.insert(name.clone(), return_type.clone());
                }
                TypedStatementKind::StructDef { name, fields } => {
                    self.function_params
                        .insert(name.clone(), vec![XeType::Unknown; fields.len()]);
                    self.function_returns
                        .insert(name.clone(), XeType::Struct(name.clone()));
                }
                _ => {}
            }
        }

        if !program.globals.is_empty() {
            self.emit("thread_local! {\n");
            for (name, ty) in &program.globals {
                self.emit(&format!(
                    "    static {}: RefCell<Option<{}>> = const {{ RefCell::new(None) }};\n",
                    global_key(name),
                    ty.to_rust_type()
                ));
            }
            self.emit("}\n\n");
        }

        let mut main_statements = Vec::new();
        for stmt in &program.statements {
            match &stmt.kind {
                TypedStatementKind::FunctionDef {
                    name,
                    params,
                    locals,
                    body,
                    return_type,
                } => self.generate_function(name, params, locals, body, return_type),
                TypedStatementKind::StructDef { name, fields } => self.generate_struct(name, fields),
                _ => main_statements.push(stmt),
            }
        }

        self.line("fn xe_main() {");
        self.indent_level += 1;
        self.scopes = vec![HashMap::new()];
        for stmt in main_statements {
            self.generate_statement(stmt);
        }
        self.indent_level -= 1;
        self.line("}");
        self.emit("\n");

        let builtins = std::mem::take(&mut self.builtin_values);
        for builtin in builtins {
            self.generate_builtin_value(builtin);
        }

        self.emit("static XE_LOCATIONS: &[(&str, usize, &str)] = &[\n");
        let locations = std::mem::take(&mut self.locations);
        for (file, line, text) in &locations {
            self.emit(&format!("    ({file:?}, {line}, {text:?}),\n"));
        }
        self.emit("];\n\n");
        self.emit("fn main() {\n    xe_start(xe_main);\n}\n");

        std::mem::take(&mut self.output)
    }

    fn generate_function(
        &mut self,
        name: &str,
        params: &[(String, XeType)],
        locals: &[(String, XeType)],
        body: &[TypedStatement],
        return_type: &XeType,
    ) {
        let signature = params
            .iter()
            .map(|(p, ty)| format!("mut {}: {}", local_name(p), ty.to_rust_type()))
            .collect::<Vec<_>>()
            .join(", ");
        self.line(&format!(
            "fn {}({}) -> {} {{",
            name,
            signature,
            return_type.to_rust_type()
        ));
        self.indent_level += 1;

        // Variables are function-scoped (as in Python), so all of them are declared up
        // front. The defaults are never read: the analyzer rejects reads of variables
        // that might not have been assigned.
        let mut scope: HashMap<String, XeType> = params.iter().cloned().collect();
        for (local, ty) in locals {
            self.line(&format!(
                "let mut {}: {} = {};",
                local_name(local),
                ty.to_rust_type(),
                default_value(ty)
            ));
            scope.insert(local.clone(), ty.clone());
        }

        let saved_scopes = std::mem::replace(&mut self.scopes, vec![scope]);
        let saved_return = std::mem::replace(&mut self.return_type, return_type.clone());
        let saved_try_depth = std::mem::replace(&mut self.try_depth, 0);
        let saved_loops = std::mem::take(&mut self.loop_try_depths);

        for s in body {
            self.generate_statement(s);
        }
        // Value for paths that fall off the end of the body.
        match return_type {
            XeType::Void => {}
            ty if ty.is_dynamic() => self.line("XeValue::None"),
            _ => self.line("unreachable!()"),
        }

        self.loop_try_depths = saved_loops;
        self.try_depth = saved_try_depth;
        self.return_type = saved_return;
        self.scopes = saved_scopes;
        self.indent_level -= 1;
        self.line("}");

        // The function as a value, callable with dynamic arguments.
        let args = params
            .iter()
            .enumerate()
            .map(|(i, (_, ty))| convert(&format!("__xe_args[{i}].clone()"), &XeType::Unknown, ty))
            .collect::<Vec<_>>()
            .join(", ");
        let call = convert(&format!("{name}({args})"), return_type, &XeType::Unknown);
        self.line(&cached_function_value(
            &function_value_name(name),
            &format!(
                "xe_function({:?}, {n}, Some({n}), Box::new(|__xe_args: Vec<XeValue>| -> XeValue {{ {} }}))",
                display_name(name),
                call,
                n = params.len()
            ),
        ));
        self.emit("\n");
    }

    fn generate_struct(&mut self, name: &str, fields: &[String]) {
        let params = (0..fields.len())
            .map(|i| format!("a{i}: XeValue"))
            .collect::<Vec<_>>()
            .join(", ");
        let field_literals = fields
            .iter()
            .map(|f| format!("{f:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let field_values = (0..fields.len())
            .map(|i| format!("a{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        self.line(&format!("fn {name}({params}) -> XeValue {{"));
        self.line(&format!(
            "    xe_make_struct({:?}, &[{}], vec![{}])",
            display_name(name),
            field_literals,
            field_values
        ));
        self.line("}");
        let args = (0..fields.len())
            .map(|i| format!("__xe_args[{i}].clone()"))
            .collect::<Vec<_>>()
            .join(", ");
        self.line(&cached_function_value(
            &function_value_name(name),
            &format!(
                "xe_function({:?}, {n}, Some({n}), Box::new(|__xe_args: Vec<XeValue>| -> XeValue {{ {}({}) }}))",
                display_name(name),
                name,
                args,
                n = fields.len()
            ),
        ));
        self.emit("\n");
    }

    /// A builtin used as a value, e.g. `apply(upper, words)`.
    fn generate_builtin_value(&mut self, builtin: Builtin) {
        let signature = builtin.signature();
        let body = match signature.max_args {
            None => {
                let call = match builtin {
                    Builtin::Print => "{ xe_builtin_print(__xe_args); XeValue::None }",
                    Builtin::Min => "xe_builtin_min(__xe_args)",
                    Builtin::Max => "xe_builtin_max(__xe_args)",
                    _ => unreachable!("only print, min and max are variadic"),
                };
                call.to_string()
            }
            Some(max) => {
                let saved_scopes = std::mem::replace(&mut self.scopes, vec![HashMap::new()]);
                let mut arms = Vec::new();
                for count in signature.min_args..=max {
                    let mut binds = String::new();
                    let mut args = Vec::new();
                    for i in 0..count {
                        let name = format!("__xe_arg{i}");
                        binds.push_str(&format!("let {} = __xe_args[{}].clone(); ", local_name(&name), i));
                        self.scopes[0].insert(name.clone(), XeType::Unknown);
                        let arg = TypedExpression {
                            kind: TypedExpressionKind::Identifier(name),
                            ty: XeType::Unknown,
                            span: Span::new(0, 0),
                        };
                        let expected = signature.param_type(i);
                        args.push(if expected.is_dynamic() {
                            arg
                        } else {
                            TypedExpression {
                                ty: expected.clone(),
                                span: Span::new(0, 0),
                                kind: TypedExpressionKind::Unwrap(Box::new(arg), expected),
                            }
                        });
                    }
                    let call = TypedExpression {
                        kind: TypedExpressionKind::BuiltinCall { builtin, args },
                        ty: builtin_result_type(builtin, &signature.return_type),
                        span: Span::new(0, 0),
                    };
                    let code = self.expr_as(&call, &XeType::Unknown);
                    arms.push(format!("{count} => {{ {binds}{code} }}"));
                }
                self.scopes = saved_scopes;
                format!(
                    "match __xe_args.len() {{ {}, _ => unreachable!() }}",
                    arms.join(", ")
                )
            }
        };
        let max = match signature.max_args {
            Some(max) => format!("Some({max})"),
            None => "None".to_string(),
        };
        self.line(&cached_function_value(
            &builtin_value_name(builtin),
            &format!(
                "xe_function({:?}, {}, {}, Box::new(|__xe_args: Vec<XeValue>| -> XeValue {{ {} }}))",
                builtin.name(),
                signature.min_args,
                max,
                body
            ),
        ));
        self.emit("\n");
    }

    fn location_id(&mut self, span: &Span) -> usize {
        let Some(file) = &span.source_name else {
            return 0;
        };
        let key = (file.to_string(), span.line);
        if let Some(id) = self.location_ids.get(&key) {
            return *id;
        }
        let text = self
            .sources
            .get(&**file)
            .and_then(|source| source.lines().nth(span.line.saturating_sub(1)))
            .unwrap_or_default()
            .to_string();
        let id = self.locations.len();
        self.locations.push((file.to_string(), span.line, text));
        self.location_ids.insert(key, id);
        id
    }

    fn generate_block(&mut self, statements: &[TypedStatement]) {
        self.indent_level += 1;
        for s in statements {
            self.generate_statement(s);
        }
        self.indent_level -= 1;
    }

    fn generate_statement(&mut self, stmt: &TypedStatement) {
        if matches!(stmt.kind, TypedStatementKind::Nop) {
            return;
        }
        self.current_location = self.location_id(&stmt.span);
        if !matches!(stmt.kind, TypedStatementKind::While { .. }) && self.current_location != 0 {
            self.line(&format!("xe_loc({});", self.current_location));
        }

        match &stmt.kind {
            TypedStatementKind::Assignment { name, value } => {
                let code = self.assign_code(name, value);
                self.line(&code);
            }
            TypedStatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                let cond = self.expr_as(condition, &XeType::Boolean);
                self.line(&format!("if {cond} {{"));
                self.generate_block(then_block);
                if let Some(else_stmts) = else_block {
                    self.line("} else {");
                    self.generate_block(else_stmts);
                }
                self.line("}");
            }
            TypedStatementKind::While { condition, body } => {
                let location = self.current_location;
                let cond = self.expr_as(condition, &XeType::Boolean);
                self.line(&format!("while {{ xe_loc({location}); {cond} }} {{"));
                self.generate_loop_body(body, None);
                self.line("}");
            }
            TypedStatementKind::Repeat { count, body } => {
                let count = self.expr_as(count, &XeType::Number);
                self.line(&format!("for _ in 0..xe_repeat_count({count}) {{"));
                self.generate_loop_body(body, None);
                self.line("}");
            }
            TypedStatementKind::For {
                variable,
                variable_type,
                iterable,
                body,
            } => {
                let range_args = match &iterable.kind {
                    TypedExpressionKind::BuiltinCall {
                        builtin: Builtin::Range,
                        args,
                    } => Some(args),
                    _ => None,
                };
                let binding = match range_args {
                    Some(args) => {
                        let args = args
                            .iter()
                            .map(|arg| self.expr_as(arg, &XeType::Number))
                            .collect::<Vec<_>>();
                        let (start, stop, step) = match args.as_slice() {
                            [stop] => ("0.0".to_string(), stop.clone(), "1.0".to_string()),
                            [start, stop] => (start.clone(), stop.clone(), "1.0".to_string()),
                            [start, stop, step] => (start.clone(), stop.clone(), step.clone()),
                            _ => unreachable!("range() takes 1 to 3 arguments"),
                        };
                        self.line(&format!(
                            "for __xe_n in xe_range({start}, {stop}, {step}) {{"
                        ));
                        convert("__xe_n", &XeType::Number, variable_type)
                    }
                    None => {
                        let iterable = self.expr_as(iterable, &XeType::Unknown);
                        self.line(&format!("for __xe_item in xe_iter(&{iterable}) {{"));
                        convert("__xe_item", &XeType::Unknown, variable_type)
                    }
                };
                self.generate_loop_body(body, Some((variable, variable_type, binding)));
                self.line("}");
            }
            TypedStatementKind::Return { value } => {
                let return_type = self.return_type.clone();
                let value = match (value, &return_type) {
                    (Some(expr), XeType::Void) => {
                        let code = self.expr(expr);
                        format!("{{ let _ = {code}; }}")
                    }
                    (Some(expr), ty) => self.expr_as(expr, ty),
                    (None, ty) => convert("XeValue::None", &XeType::Unknown, ty),
                };
                if self.try_depth > 0 {
                    self.line(&format!("return XeFlow::Return({value});"));
                } else {
                    self.line(&format!("return {value};"));
                }
            }
            TypedStatementKind::Break => {
                let code = self.loop_exit("Break");
                self.line(&code);
            }
            TypedStatementKind::Continue => {
                let code = self.loop_exit("Continue");
                self.line(&code);
            }
            TypedStatementKind::Expression(expr) => {
                let code = self.expr(expr);
                self.line(&format!("let _ = {code};"));
            }
            TypedStatementKind::IndexAssignment {
                object,
                index,
                value,
                op,
            } => {
                // Python order: the value, then the object, then the index.
                let value = self.expr_as(value, &XeType::Unknown);
                let object = self.expr_as(object, &XeType::Unknown);
                let index = self.expr_as(index, &XeType::Unknown);
                let update = match op {
                    Some(op) => format!(
                        "let __xe_v = xe_binary({:?}, xe_index(&__xe_o, &__xe_k), __xe_v); ",
                        op.symbol()
                    ),
                    None => String::new(),
                };
                self.line(&format!(
                    "{{ let __xe_v = {value}; let __xe_o = {object}; let __xe_k = {index}; {update}xe_set_index(&__xe_o, __xe_k, __xe_v); }}"
                ));
            }
            TypedStatementKind::FieldAssignment {
                object,
                field,
                value,
                op,
            } => {
                let value = self.expr_as(value, &XeType::Unknown);
                let object = self.expr_as(object, &XeType::Unknown);
                let update = match op {
                    Some(op) => format!(
                        "let __xe_v = xe_binary({:?}, xe_get_field(&__xe_o, {:?}), __xe_v); ",
                        op.symbol(),
                        field
                    ),
                    None => String::new(),
                };
                self.line(&format!(
                    "{{ let __xe_v = {value}; let __xe_o = {object}; {update}xe_set_field(&__xe_o, {field:?}, __xe_v); }}"
                ));
            }
            TypedStatementKind::Try {
                body,
                catch_variable,
                handler,
            } => self.generate_try(body, catch_variable.as_deref(), handler),
            TypedStatementKind::Nop
            | TypedStatementKind::FunctionDef { .. }
            | TypedStatementKind::StructDef { .. } => {}
        }
    }

    fn assign_code(&mut self, name: &str, value: &TypedExpression) -> String {
        if let Some(code) = self.append_code(name, value) {
            return code;
        }
        if let Some(ty) = self.local_type(name) {
            let code = self.expr_as(value, &ty);
            format!("{} = {};", local_name(name), code)
        } else if let Some(ty) = self.globals.get(name).cloned() {
            let code = self.expr_as(value, &ty);
            format!("xe_gset(&{}, {});", global_key(name), code)
        } else {
            unreachable!("assignment to undeclared variable '{}'", name)
        }
    }

    fn append_code(&mut self, name: &str, value: &TypedExpression) -> Option<String> {
        let TypedExpressionKind::BinaryOp {
            left,
            op: BinaryOperator::Add,
            right,
        } = &value.kind
        else {
            return None;
        };
        let appends_to_self = matches!(&left.kind, TypedExpressionKind::Identifier(n) if n == name);
        if !appends_to_self || left.ty != XeType::Text || right.ty != XeType::Text {
            return None;
        }
        let text = match &right.kind {
            TypedExpressionKind::String(literal) => format!("{literal:?}"),
            _ => self.expr_as(right, &XeType::Text),
        };
        if self.local_type(name) == Some(XeType::Text) {
            return Some(format!(
                "{{ let __xe_v = {}; {}.push_str(&__xe_v); }}",
                text,
                local_name(name)
            ));
        }
        if self.globals.get(name) == Some(&XeType::Text) && !has_user_calls(right) {
            let key = global_key(name);
            return Some(format!(
                "{{ xe_gref(&{}, {:?}, |_| ()); let __xe_v = {}; xe_gappend(&{}, &__xe_v); }}",
                key,
                display_name(name),
                text,
                key
            ));
        }
        None
    }

    /// Module-level variables are borrowed only while no later operand can run user code
    /// that might reassign them.
    fn with_operands(
        &mut self,
        operands: &[(&TypedExpression, XeType, Pass)],
        body: impl FnOnce(&mut Self, &[String]) -> String,
    ) -> String {
        let borrows: Vec<Option<Borrow>> = operands
            .iter()
            .enumerate()
            .map(|(i, (expr, target, pass))| {
                let later_pure = operands[i + 1..].iter().all(|(e, _, _)| !has_user_calls(e));
                self.borrowable(expr, target, *pass, later_pure)
            })
            .collect();
        let last_global = borrows
            .iter()
            .rposition(|borrow| matches!(borrow, Some(Borrow::Global { .. })));

        let mut prefix = String::new();
        let mut suffix = String::new();
        let mut refs = Vec::with_capacity(operands.len());
        for (i, ((expr, target, _), borrow)) in operands.iter().zip(borrows).enumerate() {
            match borrow {
                Some(Borrow::Place(place)) => refs.push(place),
                Some(Borrow::Global { key, name }) => {
                    self.temp_counter += 1;
                    let reference = format!("__xe_ref{}", self.temp_counter);
                    prefix.push_str(&format!("xe_gref(&{key}, {name:?}, |{reference}| {{ "));
                    suffix.push_str(" })");
                    refs.push(format!("(*{reference})"));
                }
                None => {
                    let code = self.expr_as(expr, target);
                    if last_global.is_some_and(|last| i < last) {
                        self.temp_counter += 1;
                        let temp = format!("__xe_t{}", self.temp_counter);
                        prefix.push_str(&format!("let {temp} = {code}; "));
                        refs.push(temp);
                    } else {
                        refs.push(format!("({code})"));
                    }
                }
            }
        }
        let body = body(self, &refs);
        if prefix.is_empty() {
            body
        } else {
            format!("{{ {prefix}{body}{suffix} }}")
        }
    }

    fn borrowable(
        &self,
        expr: &TypedExpression,
        target: &XeType,
        pass: Pass,
        later_pure: bool,
    ) -> Option<Borrow> {
        let same_kind = |ty: &XeType| ty == target || (ty.is_dynamic() && target.is_dynamic());
        match (&expr.kind, pass) {
            (_, Pass::Owned) => None,
            (TypedExpressionKind::String(literal), Pass::RefOrStr) if *target == XeType::Text => {
                Some(Borrow::Place(format!("*{literal:?}")))
            }
            (TypedExpressionKind::Identifier(name), _) if same_kind(&expr.ty) => {
                if self.local_type(name).is_some() {
                    return Some(Borrow::Place(local_name(name)));
                }
                let stored = self.globals.get(name)?;
                (later_pure && same_kind(stored)).then(|| Borrow::Global {
                    key: global_key(name),
                    name: display_name(name).to_string(),
                })
            }
            _ => None,
        }
    }

    fn generate_loop_body(
        &mut self,
        body: &[TypedStatement],
        variable: Option<(&String, &XeType, String)>,
    ) {
        self.indent_level += 1;
        self.loop_try_depths.push(self.try_depth);
        let has_variable = variable.is_some();
        if let Some((name, ty, binding)) = variable {
            self.line(&format!(
                "let mut {}: {} = {};",
                local_name(name),
                ty.to_rust_type(),
                binding
            ));
            self.scopes.push(HashMap::from([(name.clone(), ty.clone())]));
        }
        for s in body {
            self.generate_statement(s);
        }
        if has_variable {
            self.scopes.pop();
        }
        self.loop_try_depths.pop();
        self.indent_level -= 1;
    }

    /// `break` or `continue`; inside a `try` body (a closure) it is passed out as a value.
    fn loop_exit(&self, kind: &str) -> String {
        match self.loop_try_depths.last() {
            Some(depth) if *depth == self.try_depth => format!("{};", kind.to_lowercase()),
            Some(_) => format!("return XeFlow::{kind};"),
            None => "unreachable!()".to_string(),
        }
    }

    /// `try` runs its body in a closure under `catch_unwind`; runtime errors are panics.
    /// `return`, `break` and `continue` inside the body leave the closure as `XeFlow`.
    fn generate_try(
        &mut self,
        body: &[TypedStatement],
        catch_variable: Option<&str>,
        handler: &[TypedStatement],
    ) {
        self.temp_counter += 1;
        let flow = format!("__xe_flow{}", self.temp_counter);
        let result_type = self.return_type.to_rust_type();

        self.line(&format!(
            "let {flow}: XeFlow<{result_type}> = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> XeFlow<{result_type}> {{"
        ));
        self.try_depth += 1;
        self.generate_block(body);
        self.try_depth -= 1;
        self.line("    XeFlow::Normal");
        self.line("})) {");
        self.line("    Ok(flow) => flow,");
        self.line("    Err(payload) => {");
        self.indent_level += 1;
        self.line("    let __xe_message = xe_panic_message(payload);");
        if let Some(name) = catch_variable {
            let message = TypedExpression {
                kind: TypedExpressionKind::Identifier("__xe_message".to_string()),
                ty: XeType::Text,
                span: Span::new(0, 0),
            };
            self.scopes.push(HashMap::from([("__xe_message".to_string(), XeType::Text)]));
            let code = self.assign_code(name, &message);
            self.scopes.pop();
            // `l___xe_message` is the local-name form of the message binding.
            self.line(&format!("    let {} = __xe_message;", local_name("__xe_message")));
            self.line(&format!("    {code}"));
        }
        self.generate_block(handler);
        self.line("    XeFlow::Normal");
        self.indent_level -= 1;
        self.line("    }");
        self.line("};");

        let break_arm = self.loop_exit("Break");
        let continue_arm = self.loop_exit("Continue");
        let return_arm = if self.try_depth > 0 {
            "return XeFlow::Return(__xe_r);"
        } else {
            "return __xe_r;"
        };
        self.line(&format!(
            "match {flow} {{ XeFlow::Normal => {{}} XeFlow::Break => {{ {break_arm} }} XeFlow::Continue => {{ {continue_arm} }} XeFlow::Return(__xe_r) => {{ {return_arm} }} }}"
        ));
    }

    /// Generates `expr` and converts the result to `target`.
    fn expr_as(&mut self, expr: &TypedExpression, target: &XeType) -> String {
        let code = self.expr(expr);
        convert(&code, &expr.ty, target)
    }

    /// Re-marks the current statement after a call returns, so later errors in the
    /// statement are not reported at a line inside the callee.
    fn after_call(&self, call: String) -> String {
        if self.current_location == 0 {
            call
        } else {
            format!(
                "{{ let __xe_r = {}; xe_loc({}); __xe_r }}",
                call, self.current_location
            )
        }
    }

    /// Generates a Rust expression whose type is `expr.ty.to_rust_type()`.
    fn expr(&mut self, expr: &TypedExpression) -> String {
        match &expr.kind {
            TypedExpressionKind::Number(n) => format!("{n:?}f64"),
            TypedExpressionKind::String(s) => format!("{s:?}.to_string()"),
            TypedExpressionKind::Boolean(b) => b.to_string(),
            TypedExpressionKind::None => "XeValue::None".to_string(),
            TypedExpressionKind::List(elements) => {
                let items = elements
                    .iter()
                    .map(|e| self.expr_as(e, &XeType::Unknown))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("xe_list(vec![{items}])")
            }
            TypedExpressionKind::Map(entries) => {
                let entries = entries
                    .iter()
                    .map(|(k, v)| {
                        let k = self.expr_as(k, &XeType::Unknown);
                        let v = self.expr_as(v, &XeType::Unknown);
                        format!("({k}, {v})")
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("xe_make_map(vec![{entries}])")
            }
            TypedExpressionKind::Identifier(name) => {
                if self.local_type(name).is_some() {
                    format!("{}.clone()", local_name(name))
                } else if let Some(ty) = self.globals.get(name).cloned() {
                    let read = format!("xe_gget(&{}, {:?})", global_key(name), display_name(name));
                    convert(&read, &ty, &expr.ty)
                } else {
                    unreachable!("undeclared variable '{}'", name)
                }
            }
            TypedExpressionKind::FunctionRef(name) => format!("{}()", function_value_name(name)),
            TypedExpressionKind::BuiltinRef(builtin) => {
                if !self.builtin_values.contains(builtin) {
                    self.builtin_values.push(*builtin);
                }
                format!("{}()", builtin_value_name(*builtin))
            }
            TypedExpressionKind::BinaryOp { left, op, right } => self.binary_op(expr, left, *op, right),
            TypedExpressionKind::UnaryOp { op, operand } => match op {
                UnaryOperator::Negate => format!("(-{})", self.expr_as(operand, &XeType::Number)),
                UnaryOperator::Not => format!("(!{})", self.expr_as(operand, &XeType::Boolean)),
            },
            TypedExpressionKind::FunctionCall { name, args } => {
                let param_types = self.function_params.get(name).cloned().unwrap_or_default();
                let args = args
                    .iter()
                    .zip(param_types.iter())
                    .map(|(arg, ty)| self.expr_as(arg, ty))
                    .collect::<Vec<_>>()
                    .join(", ");
                let call = self.after_call(format!("{name}({args})"));
                // A call analyzed before its callee sees a provisional `unknown` type.
                let actual = self.function_returns.get(name).cloned().unwrap_or(XeType::Unknown);
                convert(&call, &actual, &expr.ty)
            }
            TypedExpressionKind::BuiltinCall { builtin, args } => {
                let code = self.builtin_call(*builtin, args);
                let actual = builtin_result_type(*builtin, &builtin.signature().return_type);
                convert(&code, &actual, &expr.ty)
            }
            TypedExpressionKind::DynamicCall { callee, args } => {
                let callee = self.expr_as(callee, &XeType::Unknown);
                let args = args
                    .iter()
                    .map(|arg| self.expr_as(arg, &XeType::Unknown))
                    .collect::<Vec<_>>()
                    .join(", ");
                let call = self.after_call(format!("xe_call(&{callee}, vec![{args}])"));
                convert(&call, &XeType::Unknown, &expr.ty)
            }
            TypedExpressionKind::Lambda {
                params,
                captures,
                body,
            } => self.lambda(params, captures, body),
            TypedExpressionKind::Index { object, index } => {
                let (target, function) = if object.ty == XeType::Text {
                    (XeType::Text, "xe_text_index")
                } else {
                    (XeType::Unknown, "xe_index")
                };
                let code = self.with_operands(
                    &[(object, target, Pass::Ref), (index, XeType::Unknown, Pass::Ref)],
                    |_, r| format!("{}(&{}, &{})", function, r[0], r[1]),
                );
                convert(&code, &XeType::Unknown, &expr.ty)
            }
            TypedExpressionKind::Slice { object, start, end } => {
                let is_text = object.ty == XeType::Text;
                let target = if is_text { XeType::Text } else { XeType::Unknown };
                let mut operands = vec![(&**object, target, Pass::Ref)];
                operands.extend(start.iter().map(|b| (&**b, XeType::Number, Pass::Owned)));
                operands.extend(end.iter().map(|b| (&**b, XeType::Number, Pass::Owned)));
                let code = self.with_operands(&operands, |_, r| {
                    let mut bounds = r[1..].iter().map(|b| format!("Some({b})"));
                    let mut bound = |present: bool| {
                        if present {
                            bounds.next().unwrap()
                        } else {
                            "None".to_string()
                        }
                    };
                    let (start, end) = (bound(start.is_some()), bound(end.is_some()));
                    if is_text {
                        format!("XeValue::Text(xe_text_slice(&{}, {}, {}))", r[0], start, end)
                    } else {
                        format!("xe_slice(&{}, {}, {})", r[0], start, end)
                    }
                });
                convert(&code, &XeType::Unknown, &expr.ty)
            }
            TypedExpressionKind::FieldAccess { object, field } => {
                let object = self.expr_as(object, &XeType::Unknown);
                convert(
                    &format!("xe_get_field(&{object}, {field:?})"),
                    &XeType::Unknown,
                    &expr.ty,
                )
            }
            TypedExpressionKind::Truthy(inner) if inner.ty == XeType::Text => self.with_operands(
                &[(inner, XeType::Text, Pass::Ref)],
                |_, r| format!("!{}.is_empty()", r[0]),
            ),
            TypedExpressionKind::Truthy(inner) => {
                let code = self.expr(inner);
                match &inner.ty {
                    XeType::Boolean => code,
                    XeType::Number => format!("({code} != 0.0)"),
                    XeType::Void => format!("{{ {code}; false }}"),
                    _ => format!("xe_truthy(&{code})"),
                }
            }
            TypedExpressionKind::Wrap(inner) => self.expr_as(inner, &expr.ty),
            TypedExpressionKind::Unwrap(inner, ty) => self.expr_as(inner, ty),
        }
    }

    fn lambda(
        &mut self,
        params: &[String],
        captures: &[(String, XeType)],
        body: &TypedExpression,
    ) -> String {
        let copies = captures
            .iter()
            .map(|(name, _)| format!("let __xe_c_{} = {}.clone(); ", name, local_name(name)))
            .collect::<String>();

        let mut scope: HashMap<String, XeType> = captures.iter().cloned().collect();
        let mut binds = String::new();
        for (i, param) in params.iter().enumerate() {
            scope.insert(param.clone(), XeType::Unknown);
            binds.push_str(&format!("let mut {} = __xe_args[{}].clone(); ", local_name(param), i));
        }
        for (name, _) in captures {
            if !params.contains(name) {
                binds.push_str(&format!("let mut {} = __xe_c_{}.clone(); ", local_name(name), name));
            }
        }

        let saved_scopes = std::mem::replace(&mut self.scopes, vec![scope]);
        let body = self.expr_as(body, &XeType::Unknown);
        self.scopes = saved_scopes;

        format!(
            "{{ {}xe_function(\"<lambda>\", {n}, Some({n}), Box::new(move |__xe_args: Vec<XeValue>| -> XeValue {{ xe_loc({}); {}{} }})) }}",
            copies,
            self.current_location,
            binds,
            body,
            n = params.len()
        )
    }

    fn binary_op(
        &mut self,
        expr: &TypedExpression,
        left: &TypedExpression,
        op: BinaryOperator,
        right: &TypedExpression,
    ) -> String {
        use XeType::{Boolean, Number, Text, Unknown};
        let both = |ty: &XeType| left.ty == *ty && right.ty == *ty;
        let symbol = op.symbol();
        match op {
            BinaryOperator::Add if both(&Text) => self.with_operands(
                &[(left, Text, Pass::Owned), (right, Text, Pass::RefOrStr)],
                |_, r| format!("xe_concat({}, &{})", r[0], r[1]),
            ),
            BinaryOperator::Add if !both(&Number) => {
                let code = self.with_operands(
                    &[(left, Unknown, Pass::Owned), (right, Unknown, Pass::Owned)],
                    |_, r| format!("xe_add({}, {})", r[0], r[1]),
                );
                convert(&code, &Unknown, &expr.ty)
            }
            BinaryOperator::Less
            | BinaryOperator::Greater
            | BinaryOperator::LessEqual
            | BinaryOperator::GreaterEqual
                if both(&Text) =>
            {
                self.with_operands(
                    &[(left, Text, Pass::Ref), (right, Text, Pass::Ref)],
                    |_, r| format!("({} {} {})", r[0], symbol, r[1]),
                )
            }
            BinaryOperator::Less
            | BinaryOperator::Greater
            | BinaryOperator::LessEqual
            | BinaryOperator::GreaterEqual
                if !both(&Number) =>
            {
                self.with_operands(
                    &[(left, Unknown, Pass::Ref), (right, Unknown, Pass::Ref)],
                    |_, r| format!("xe_compare(&{}, &{}, {:?})", r[0], r[1], symbol),
                )
            }
            BinaryOperator::Equal | BinaryOperator::NotEqual if !both(&Number) && !both(&Boolean) => {
                let negate = if op == BinaryOperator::NotEqual { "!" } else { "" };
                if both(&Text) {
                    self.with_operands(
                        &[(left, Text, Pass::RefOrStr), (right, Text, Pass::RefOrStr)],
                        |_, r| format!("({}({} == {}))", negate, r[0], r[1]),
                    )
                } else {
                    self.with_operands(
                        &[(left, Unknown, Pass::Ref), (right, Unknown, Pass::Ref)],
                        |_, r| format!("({}xe_eq(&{}, &{}))", negate, r[0], r[1]),
                    )
                }
            }
            BinaryOperator::In | BinaryOperator::NotIn => {
                let negate = if op == BinaryOperator::NotIn { "!" } else { "" };
                self.with_operands(
                    &[(left, left.ty.clone(), Pass::Ref), (right, right.ty.clone(), Pass::Ref)],
                    |_, r| format!("({}xe_in(&{}, &{}))", negate, r[0], r[1]),
                )
            }
            _ => {
                let l = self.expr(left);
                let r = self.expr(right);
                match op {
                    BinaryOperator::Add => format!("({l} + {r})"),
                    BinaryOperator::Subtract => format!("({l} - {r})"),
                    BinaryOperator::Multiply => format!("({l} * {r})"),
                    BinaryOperator::Divide => format!("xe_div({l}, {r})"),
                    BinaryOperator::FloorDivide => format!("xe_floor_div({l}, {r})"),
                    BinaryOperator::Modulo => format!("xe_mod({l}, {r})"),
                    BinaryOperator::Power => format!("xe_pow({l}, {r})"),
                    BinaryOperator::Equal if both(&Number) => format!("xe_num_eq({l}, {r})"),
                    BinaryOperator::NotEqual if both(&Number) => format!("(!xe_num_eq({l}, {r}))"),
                    BinaryOperator::And => format!("({l} && {r})"),
                    BinaryOperator::Or => format!("({l} || {r})"),
                    _ => format!("({l} {symbol} {r})"),
                }
            }
        }
    }

    fn builtin_call(&mut self, builtin: Builtin, args: &[TypedExpression]) -> String {
        if builtin == Builtin::Length
            && let TypedExpressionKind::Wrap(inner) = &args[0].kind
                && inner.ty == XeType::Text {
                    return self.with_operands(&[(inner, XeType::Text, Pass::Ref)], |_, r| {
                        format!("xe_text_length(&{})", r[0])
                    });
                }
        let by_reference = |i: usize| match builtin {
            Builtin::Print | Builtin::Min | Builtin::Max | Builtin::Exit => false,
            Builtin::Append | Builtin::Insert | Builtin::Pop => i == 0,
            Builtin::Range
            | Builtin::Abs
            | Builtin::Floor
            | Builtin::Ceil
            | Builtin::Sqrt
            | Builtin::Round
            | Builtin::Random
            | Builtin::Args => false,
            _ => true,
        };
        let operands: Vec<_> = args
            .iter()
            .enumerate()
            .map(|(i, arg)| {
                let pass = if by_reference(i) { Pass::Ref } else { Pass::Owned };
                (arg, arg.ty.clone(), pass)
            })
            .collect();
        self.with_operands(&operands, |_, a| {
            let optional = |i: usize| match a.get(i) {
                Some(code) => format!("Some({code})"),
                None => "None".to_string(),
            };
            let name = builtin.name();
            match builtin {
                Builtin::Print => format!("xe_builtin_print(vec![{}])", a.join(", ")),
                Builtin::Min | Builtin::Max => format!("xe_builtin_{}(vec![{}])", name, a.join(", ")),
                Builtin::Input => match a.first() {
                    Some(prompt) => format!("xe_builtin_input(&{prompt})"),
                    None => "xe_builtin_input(\"\")".to_string(),
                },
                Builtin::Length
                | Builtin::Type
                | Builtin::Keys
                | Builtin::Values
                | Builtin::Sum
                | Builtin::Sort
                | Builtin::Reverse
                | Builtin::Copy
                | Builtin::Upper
                | Builtin::Lower
                | Builtin::Trim
                | Builtin::ReadFile
                | Builtin::FileExists
                | Builtin::Error => format!("xe_builtin_{}(&{})", name, a[0]),
                Builtin::Convert
                | Builtin::Remove
                | Builtin::HasKey
                | Builtin::Contains
                | Builtin::Join
                | Builtin::StartsWith
                | Builtin::EndsWith
                | Builtin::Find
                | Builtin::WriteFile
                | Builtin::AppendFile => format!("xe_builtin_{}(&{}, &{})", name, a[0], a[1]),
                Builtin::Replace => format!("xe_builtin_replace(&{}, &{}, &{})", a[0], a[1], a[2]),
                Builtin::Append => format!("xe_builtin_append(&{}, {})", a[0], a[1]),
                Builtin::Insert => format!("xe_builtin_insert(&{}, {}, {})", a[0], a[1], a[2]),
                Builtin::Pop => format!("xe_builtin_pop(&{}, {})", a[0], optional(1)),
                Builtin::Split => match a.get(1) {
                    Some(separator) => {
                        format!("xe_builtin_split(&{}, Some({}.as_str()))", a[0], separator)
                    }
                    None => format!("xe_builtin_split(&{}, None)", a[0]),
                },
                Builtin::Range => match a {
                    [stop] => format!("xe_builtin_range(0.0, {stop}, 1.0)"),
                    [start, stop] => format!("xe_builtin_range({start}, {stop}, 1.0)"),
                    [start, stop, step] => format!("xe_builtin_range({start}, {stop}, {step})"),
                    _ => unreachable!("range() takes 1 to 3 arguments"),
                },
                Builtin::Abs | Builtin::Floor | Builtin::Ceil | Builtin::Sqrt => {
                    format!("xe_builtin_{}({})", name, a[0])
                }
                Builtin::Round => format!("xe_builtin_round({}, {})", a[0], optional(1)),
                Builtin::Random | Builtin::Args => format!("xe_builtin_{name}()"),
                Builtin::Exit => format!("xe_builtin_exit({})", optional(0)),
            }
        })
    }

    fn local_type(&self, name: &str) -> Option<XeType> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name)).cloned()
    }

    fn line(&mut self, s: &str) {
        for _ in 0..self.indent_level {
            self.output.push_str("    ");
        }
        self.output.push_str(s);
        self.output.push('\n');
    }

    fn emit(&mut self, s: &str) {
        self.output.push_str(s);
    }
}

#[derive(Clone, Copy)]
enum Pass {
    Owned,
    Ref,
    RefOrStr,
}

enum Borrow {
    Place(String),
    Global { key: String, name: String },
}

fn has_user_calls(expr: &TypedExpression) -> bool {
    use TypedExpressionKind as K;
    match &expr.kind {
        K::FunctionCall { .. } | K::DynamicCall { .. } => true,
        K::Number(_)
        | K::String(_)
        | K::Boolean(_)
        | K::None
        | K::Identifier(_)
        | K::FunctionRef(_)
        | K::BuiltinRef(_)
        | K::Lambda { .. } => false,
        K::List(items) => items.iter().any(has_user_calls),
        K::Map(entries) => entries
            .iter()
            .any(|(key, value)| has_user_calls(key) || has_user_calls(value)),
        K::BinaryOp { left, right, .. } => has_user_calls(left) || has_user_calls(right),
        K::BuiltinCall { args, .. } => args.iter().any(has_user_calls),
        K::Index { object, index } => has_user_calls(object) || has_user_calls(index),
        K::Slice { object, start, end } => {
            has_user_calls(object)
                || start.as_deref().is_some_and(has_user_calls)
                || end.as_deref().is_some_and(has_user_calls)
        }
        K::UnaryOp { operand: inner, .. }
        | K::FieldAccess { object: inner, .. }
        | K::Truthy(inner)
        | K::Wrap(inner)
        | K::Unwrap(inner, _) => has_user_calls(inner),
    }
}

/// The Rust type a builtin's runtime function actually returns. `convert`, `min` and
/// `max` return dynamic values even when the analyzer knows a more precise type.
fn builtin_result_type(builtin: Builtin, declared: &XeType) -> XeType {
    match builtin {
        Builtin::Convert | Builtin::Min | Builtin::Max => XeType::Unknown,
        _ => declared.clone(),
    }
}

/// Converts Rust code producing a value of type `from` into code producing type `to`.
/// Conversions out of dynamic values are checked at runtime.
fn convert(code: &str, from: &XeType, to: &XeType) -> String {
    if from == to {
        return code.to_string();
    }
    if *from == XeType::Void {
        return convert(&format!("{{ {code}; XeValue::None }}"), &XeType::Unknown, to);
    }
    if *to == XeType::Void {
        return format!("{{ let _ = {code}; }}");
    }
    match (from.is_dynamic(), to.is_dynamic()) {
        (true, true) => code.to_string(),
        (false, true) => format!("XeValue::from({code})"),
        (true, false) => match to {
            XeType::Number => format!("({code}).as_f64()"),
            XeType::Boolean => format!("({code}).as_bool()"),
            XeType::Text => format!("({code}).as_string()"),
            _ => unreachable!("only scalars are native"),
        },
        (false, false) => convert(&format!("XeValue::from({code})"), &XeType::Unknown, to),
    }
}

/// A function returning the same function value every time, so `f == f` holds.
fn cached_function_value(name: &str, value: &str) -> String {
    format!(
        "fn {name}() -> XeValue {{ thread_local! {{ static VALUE: XeValue = {value}; }} VALUE.with(|v| v.clone()) }}"
    )
}

fn default_value(ty: &XeType) -> &'static str {
    match ty {
        XeType::Number => "0.0",
        XeType::Boolean => "false",
        XeType::Text => "String::new()",
        XeType::Void => "()",
        _ => "XeValue::None",
    }
}

/// Locals get a prefix so they can never collide with Rust keywords or runtime names.
fn local_name(name: &str) -> String {
    format!("l_{name}")
}

fn global_key(name: &str) -> String {
    format!("XE_G_{name}")
}

fn function_value_name(name: &str) -> String {
    format!("xe_fnval_{name}")
}

fn builtin_value_name(builtin: Builtin) -> String {
    format!("xe_builtin_value_{}", builtin.name())
}

/// The user-facing name of a linked symbol such as `xe_m0_count`.
fn display_name(name: &str) -> &str {
    name.strip_prefix("xe_m")
        .map(|rest| rest.trim_start_matches(|c: char| c.is_ascii_digit()))
        .and_then(|rest| rest.strip_prefix('_'))
        .unwrap_or(name)
}
