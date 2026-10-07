use std::collections::HashMap;

use crate::ast::*;

pub struct CodeGenerator {
    output: String,
    indent_level: usize,
    /// Local variables in scope and their declared types.
    scopes: Vec<HashMap<String, XeType>>,
    /// Module-level variables and their storage types.
    globals: HashMap<String, XeType>,
    function_params: HashMap<String, Vec<XeType>>,
    function_returns: HashMap<String, XeType>,
    current_return_type: Option<XeType>,
}

/// The variable a place expression such as `grid[0][1]` or `p.pos.x` is rooted at.
enum PlaceRoot {
    Local(String),
    Global(String),
}

enum PlaceStep<'a> {
    Index(&'a TypedExpression),
    Field(&'a str),
}

struct Place<'a> {
    root: PlaceRoot,
    root_ty: XeType,
    steps: Vec<PlaceStep<'a>>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent_level: 0,
            scopes: Vec::new(),
            globals: HashMap::new(),
            function_params: HashMap::new(),
            function_returns: HashMap::new(),
            current_return_type: None,
        }
    }

    pub fn generate(&mut self, program: &TypedProgram) -> String {
        self.emit_prelude();

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
                    "    static {}: std::cell::RefCell<Option<{}>> = const {{ std::cell::RefCell::new(None) }};\n",
                    global_key(name),
                    ty.to_rust_type()
                ));
            }
            self.emit("}\n\n");
        }

        let mut main_statements = Vec::new();
        for stmt in &program.statements {
            if matches!(
                stmt.kind,
                TypedStatementKind::FunctionDef { .. } | TypedStatementKind::StructDef { .. }
            ) {
                self.generate_statement(stmt);
                self.emit("\n");
            } else {
                main_statements.push(stmt);
            }
        }

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
        self.emit(PRELUDE);
    }

    fn generate_block(&mut self, statements: &[TypedStatement]) {
        self.indent_level += 1;
        self.scopes.push(HashMap::new());
        for s in statements {
            self.generate_statement(s);
        }
        self.scopes.pop();
        self.indent_level -= 1;
    }

    fn generate_statement(&mut self, stmt: &TypedStatement) {
        match &stmt.kind {
            TypedStatementKind::Assignment { name, value } => {
                if let Some(ty) = self.local_type(name) {
                    let code = self.expr_as(value, &ty);
                    self.line(&format!("{} = {};", local_name(name), code));
                } else if let Some(ty) = self.globals.get(name).cloned() {
                    let code = self.expr_as(value, &ty);
                    self.line(&format!("xe_gset(&{}, {});", global_key(name), code));
                } else {
                    let ty = match &value.ty {
                        XeType::Void => XeType::Unknown,
                        ty => ty.clone(),
                    };
                    let code = self.expr_as(value, &ty);
                    self.line(&format!(
                        "let mut {}: {} = {};",
                        local_name(name),
                        ty.to_rust_type(),
                        code
                    ));
                    self.define_local(name, ty);
                }
            }
            TypedStatementKind::If {
                condition,
                then_block,
                else_block,
            } => {
                let cond = self.expr_as(condition, &XeType::Boolean);
                self.line(&format!("if ({}) {{", cond));
                self.generate_block(then_block);
                if let Some(else_stmts) = else_block {
                    self.line("} else {");
                    self.generate_block(else_stmts);
                }
                self.line("}");
            }
            TypedStatementKind::While { condition, body } => {
                let cond = self.expr_as(condition, &XeType::Boolean);
                self.line(&format!("while ({}) {{", cond));
                self.generate_block(body);
                self.line("}");
            }
            TypedStatementKind::Repeat { count, body } => {
                let count = self.expr_as(count, &XeType::Number);
                self.line(&format!(
                    "for _ in 0..xe_expect_non_negative_integer(&XeValue::from({}), \"repeat loop count\") {{",
                    count
                ));
                self.generate_block(body);
                self.line("}");
            }
            TypedStatementKind::For {
                variable,
                iterable,
                body,
            } => {
                let elem_ty = match &iterable.ty {
                    XeType::List(inner) => (**inner).clone(),
                    XeType::Text | XeType::Map | XeType::Struct(_) => XeType::Text,
                    _ => XeType::Unknown,
                };
                let (header, value_ty) = match &iterable.ty {
                    XeType::List(inner) => (self.expr(iterable), (**inner).clone()),
                    _ => {
                        let iterable = self.expr_as(iterable, &XeType::Unknown);
                        (format!("xe_iter(&{})", iterable), XeType::Unknown)
                    }
                };
                self.line(&format!("for __xe_loop_value in ({}) {{", header));
                self.indent_level += 1;
                self.scopes.push(HashMap::new());
                self.line(&format!(
                    "let mut {}: {} = {};",
                    local_name(variable),
                    elem_ty.to_rust_type(),
                    convert("__xe_loop_value", &value_ty, &elem_ty)
                ));
                self.define_local(variable, elem_ty);
                for s in body {
                    self.generate_statement(s);
                }
                self.scopes.pop();
                self.indent_level -= 1;
                self.line("}");
            }
            TypedStatementKind::FunctionDef {
                name,
                params,
                body,
                return_type,
            } => {
                let signature = params
                    .iter()
                    .map(|(p, ty)| format!("mut {}: {}", local_name(p), ty.to_rust_type()))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.line(&format!(
                    "fn {}({}) -> {} {{",
                    function_name(name),
                    signature,
                    return_type.to_rust_type()
                ));

                let saved_scopes = std::mem::replace(
                    &mut self.scopes,
                    vec![params.iter().cloned().collect()],
                );
                let saved_return = self.current_return_type.replace(return_type.clone());
                self.indent_level += 1;
                for s in body {
                    self.generate_statement(s);
                }
                // Value for paths that fall off the end of the body.
                match return_type {
                    XeType::Void => {}
                    ty if ty.is_dynamic() => self.line("XeValue::None"),
                    _ => self.line("unreachable!()"),
                }
                self.indent_level -= 1;
                self.current_return_type = saved_return;
                self.scopes = saved_scopes;
                self.line("}");
            }
            TypedStatementKind::Return { value } => {
                let return_type = self.current_return_type.clone().unwrap_or(XeType::Void);
                match (value, &return_type) {
                    (Some(expr), XeType::Void) => {
                        let code = self.expr(expr);
                        self.line(&format!("{{ let _ = {}; return; }}", code));
                    }
                    (Some(expr), ty) => {
                        let code = self.expr_as(expr, ty);
                        self.line(&format!("return {};", code));
                    }
                    (None, XeType::Void) => self.line("return;"),
                    (None, ty) => {
                        let code = convert("XeValue::None", &XeType::Unknown, ty);
                        self.line(&format!("return {};", code));
                    }
                }
            }
            TypedStatementKind::Break => self.line("break;"),
            TypedStatementKind::Continue => self.line("continue;"),
            TypedStatementKind::Nop => {}
            TypedStatementKind::Expression(expr) => {
                let code = match &expr.kind {
                    TypedExpressionKind::FunctionCall { name, args } if name == "append" => {
                        self.generate_append(args, false)
                    }
                    _ => self.expr(expr),
                };
                self.line(&format!("let _ = {};", code));
            }
            TypedStatementKind::StructDef { name, fields } => {
                let params = (0..fields.len())
                    .map(|i| format!("a{}: XeValue", i))
                    .collect::<Vec<_>>()
                    .join(", ");
                let field_literals = fields
                    .iter()
                    .map(|f| format!("{:?}", f))
                    .collect::<Vec<_>>()
                    .join(", ");
                let field_values = (0..fields.len())
                    .map(|i| format!("a{}", i))
                    .collect::<Vec<_>>()
                    .join(", ");
                self.line(&format!(
                    "fn {}({}) -> XeValue {{",
                    function_name(name),
                    params
                ));
                self.line(&format!(
                    "    xe_make_struct({:?}, &[{}], vec![{}])",
                    display_name(name),
                    field_literals,
                    field_values
                ));
                self.line("}");
            }
            TypedStatementKind::IndexAssignment {
                object,
                index,
                value,
            } => {
                let code = match self.resolve_place(object) {
                    Some(place) => {
                        let container = self.place_types(&place).map(|types| types.last().cloned());
                        match container {
                            Ok(Some(XeType::List(inner))) => {
                                let value = self.expr_as(value, &inner);
                                let index = self.expr_as(index, &XeType::Number);
                                self.place_op(
                                    &place,
                                    vec![format!("let __xe_val = {};", value)],
                                    vec![format!("let __xe_idx = {};", index)],
                                    |c, _| format!("xe_vec_set_index({}, __xe_idx, __xe_val)", c),
                                )
                            }
                            Ok(Some(ty)) if ty.is_dynamic() => {
                                let value = self.expr_as(value, &XeType::Unknown);
                                let index = self.expr_as(index, &XeType::Unknown);
                                self.place_op(
                                    &place,
                                    vec![format!("let __xe_val = {};", value)],
                                    vec![format!("let __xe_idx = {};", index)],
                                    |c, _| format!("xe_set_index({}, &__xe_idx, __xe_val)", c),
                                )
                            }
                            Ok(Some(ty)) => runtime_error(&format!(
                                "cannot assign to an element of {}",
                                ty
                            )),
                            Ok(None) => unreachable!("place types always include the root"),
                            Err(message) => runtime_error(&message),
                        }
                    }
                    None => runtime_error("invalid assignment target"),
                };
                self.line(&format!("{};", code));
            }
            TypedStatementKind::FieldAssignment {
                object,
                field,
                value,
            } => {
                let code = match self.resolve_place(object) {
                    Some(place) => {
                        let value = self.expr_as(value, &XeType::Unknown);
                        self.place_op(
                            &place,
                            vec![format!("let __xe_val = {};", value)],
                            Vec::new(),
                            |c, ty| {
                                if ty.is_dynamic() {
                                    format!("xe_set_field({}, {:?}, __xe_val)", c, field)
                                } else {
                                    runtime_error(&format!(
                                        "cannot set field '{}' on {}",
                                        field, ty
                                    ))
                                }
                            },
                        )
                    }
                    None => runtime_error("invalid assignment target"),
                };
                self.line(&format!("{};", code));
            }
        }
    }

    /// Generates `expr` and converts the result to `target`.
    fn expr_as(&mut self, expr: &TypedExpression, target: &XeType) -> String {
        let code = self.expr(expr);
        convert(&code, &expr.ty, target)
    }

    /// Generates a Rust expression whose type is `expr.ty.to_rust_type()`.
    fn expr(&mut self, expr: &TypedExpression) -> String {
        match &expr.kind {
            TypedExpressionKind::Number(n) => format!("{:?}f64", n),
            TypedExpressionKind::String(s) => format!("{:?}.to_string()", s),
            TypedExpressionKind::Boolean(b) => b.to_string(),
            TypedExpressionKind::List(elements) => {
                let inner = match &expr.ty {
                    XeType::List(inner) => (**inner).clone(),
                    _ => XeType::Unknown,
                };
                if elements.is_empty() {
                    return format!("Vec::<{}>::new()", inner.to_rust_type());
                }
                let items = elements
                    .iter()
                    .map(|elem| self.expr_as(elem, &inner))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("vec![{}]", items)
            }
            TypedExpressionKind::Identifier(name) => {
                if self.local_type(name).is_some() {
                    format!("{}.clone()", local_name(name))
                } else if let Some(ty) = self.globals.get(name).cloned() {
                    let read = format!("xe_gget(&{}, {:?})", global_key(name), display_name(name));
                    convert(&read, &ty, &expr.ty)
                } else {
                    // Semantic analysis guarantees every identifier is defined.
                    format!("{}.clone()", local_name(name))
                }
            }
            TypedExpressionKind::BinaryOp { left, op, right } => self.binary_op(expr, left, *op, right),
            TypedExpressionKind::UnaryOp { op, operand } => match op {
                UnaryOperator::Negate => format!("(-{})", self.expr_as(operand, &XeType::Number)),
                UnaryOperator::Not => format!("(!{})", self.expr_as(operand, &XeType::Boolean)),
            },
            TypedExpressionKind::FunctionCall { name, args } => self.function_call(expr, name, args),
            TypedExpressionKind::Index { object, index } => {
                if let Some(code) = self.borrowed_list_index(expr, object, index) {
                    return code;
                }
                if let XeType::List(inner) = &object.ty {
                    let object = self.expr(object);
                    let index = self.expr_as(index, &XeType::Number);
                    return convert(&format!("xe_vec_index(&{}, {})", object, index), inner, &expr.ty);
                }
                let object = self.expr_as(object, &XeType::Unknown);
                let index = self.expr_as(index, &XeType::Unknown);
                convert(
                    &format!("xe_index(&{}, &{})", object, index),
                    &XeType::Unknown,
                    &expr.ty,
                )
            }
            TypedExpressionKind::Map(entries) => {
                let entries = entries
                    .iter()
                    .map(|(k, v)| {
                        let k = self.expr_as(k, &XeType::Unknown);
                        let v = self.expr_as(v, &XeType::Unknown);
                        format!("({}.to_string(), {})", k, v)
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("xe_make_map(vec![{}])", entries)
            }
            TypedExpressionKind::FieldAccess { object, field } => {
                let object = self.expr_as(object, &XeType::Unknown);
                convert(
                    &format!("xe_get_field(&{}, {:?})", object, field),
                    &XeType::Unknown,
                    &expr.ty,
                )
            }
            TypedExpressionKind::Wrap(inner) => self.expr_as(inner, &expr.ty),
            TypedExpressionKind::Unwrap(inner, ty) => self.expr_as(inner, ty),
        }
    }

    /// `xs[i]` on a list variable reads the element without copying the whole list.
    fn borrowed_list_index(
        &mut self,
        expr: &TypedExpression,
        object: &TypedExpression,
        index: &TypedExpression,
    ) -> Option<String> {
        let TypedExpressionKind::Identifier(name) = &object.kind else {
            return None;
        };
        let (storage_ty, is_local) = match self.local_type(name) {
            Some(ty) => (ty, true),
            None => (self.globals.get(name)?.clone(), false),
        };
        let XeType::List(inner) = &storage_ty else {
            return None;
        };
        let index = self.expr_as(index, &XeType::Number);
        let read = if is_local {
            format!("{{ let __xe_i = {}; xe_vec_index(&{}, __xe_i) }}", index, local_name(name))
        } else {
            format!(
                "{{ let __xe_i = {}; xe_gwith(&{}, {:?}, |__xe_g| xe_vec_index(__xe_g, __xe_i)) }}",
                index,
                global_key(name),
                display_name(name)
            )
        };
        Some(convert(&read, inner, &expr.ty))
    }

    fn binary_op(
        &mut self,
        expr: &TypedExpression,
        left: &TypedExpression,
        op: BinaryOperator,
        right: &TypedExpression,
    ) -> String {
        let numeric = |this: &mut Self, f: &str| {
            let l = this.expr_as(left, &XeType::Number);
            let r = this.expr_as(right, &XeType::Number);
            format!("{}({}, {})", f, l, r)
        };
        let infix = |this: &mut Self, ty: &XeType, symbol: &str| {
            let l = this.expr_as(left, ty);
            let r = this.expr_as(right, ty);
            format!("({} {} {})", l, symbol, r)
        };
        match op {
            BinaryOperator::Equal | BinaryOperator::NotEqual => {
                let l = self.expr_as(left, &XeType::Unknown);
                let r = self.expr_as(right, &XeType::Unknown);
                let negate = if op == BinaryOperator::NotEqual { "!" } else { "" };
                format!("{}xe_eq(&{}, &{})", negate, l, r)
            }
            BinaryOperator::Add => {
                if left.ty == XeType::Number && right.ty == XeType::Number {
                    infix(self, &XeType::Number, "+")
                } else {
                    let l = self.expr_as(left, &XeType::Unknown);
                    let r = self.expr_as(right, &XeType::Unknown);
                    convert(
                        &format!("xe_add_dynamic({}, {})", l, r),
                        &XeType::Unknown,
                        &expr.ty,
                    )
                }
            }
            BinaryOperator::Subtract => numeric(self, "xe_sub_native"),
            BinaryOperator::Multiply => numeric(self, "xe_mul_native"),
            BinaryOperator::Divide => numeric(self, "xe_div_native"),
            BinaryOperator::Modulo => numeric(self, "xe_mod_native"),
            BinaryOperator::Less => infix(self, &XeType::Number, "<"),
            BinaryOperator::Greater => infix(self, &XeType::Number, ">"),
            BinaryOperator::LessEqual => infix(self, &XeType::Number, "<="),
            BinaryOperator::GreaterEqual => infix(self, &XeType::Number, ">="),
            BinaryOperator::And => infix(self, &XeType::Boolean, "&&"),
            BinaryOperator::Or => infix(self, &XeType::Boolean, "||"),
        }
    }

    fn function_call(&mut self, expr: &TypedExpression, name: &str, args: &[TypedExpression]) -> String {
        let unknown = XeType::Unknown;
        match name {
            "print" => {
                let args = args
                    .iter()
                    .map(|arg| self.expr_as(arg, &unknown))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("xe_builtin_print(vec![{}])", args)
            }
            "input" => {
                let prompt = match args.first() {
                    Some(arg) => self.expr_as(arg, &XeType::Text),
                    None => "String::new()".to_string(),
                };
                format!("xe_builtin_input(&{})", prompt)
            }
            "length" | "type" | "keys" | "values" => {
                let arg = self.expr_as(&args[0], &unknown);
                format!("xe_builtin_{}(&{})", name, arg)
            }
            "convert" => {
                let value = self.expr_as(&args[0], &unknown);
                let target = self.expr_as(&args[1], &XeType::Text);
                format!("xe_builtin_convert(&{}, &{})", value, target)
            }
            "has_key" | "contains" => {
                let target = self.expr_as(&args[0], &unknown);
                let item = self.expr_as(&args[1], &unknown);
                format!("xe_builtin_{}(&{}, &{})", name, target, item)
            }
            "split" => {
                let text = self.expr_as(&args[0], &XeType::Text);
                let delim = self.expr_as(&args[1], &XeType::Text);
                format!("xe_builtin_split(&{}, &{})", text, delim)
            }
            "join" => {
                let list = self.expr_as(&args[0], &unknown);
                let delim = self.expr_as(&args[1], &XeType::Text);
                format!("xe_builtin_join(&{}.as_list(), &{})", list, delim)
            }
            "append" => {
                let code = self.generate_append(args, true);
                convert(&code, &unknown, &expr.ty)
            }
            "pop" => {
                let code = self.generate_pop(args);
                convert(&code, &unknown, &expr.ty)
            }
            _ => {
                let param_types = self
                    .function_params
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| vec![XeType::Unknown; args.len()]);
                let args = args
                    .iter()
                    .zip(param_types.iter())
                    .map(|(arg, ty)| self.expr_as(arg, ty))
                    .collect::<Vec<_>>()
                    .join(", ");
                let call = format!("{}({})", function_name(name), args);
                // A call analyzed before its callee was sees a provisional `unknown` type.
                let actual = self.function_returns.get(name).cloned().unwrap_or(XeType::Unknown);
                convert(&call, &actual, &expr.ty)
            }
        }
    }

    /// `append(list, item)` mutates the list in place; as an expression it evaluates to the list.
    fn generate_append(&mut self, args: &[TypedExpression], want_value: bool) -> String {
        let Some(place) = self.resolve_place(strip_coercions(&args[0])) else {
            let list = self.expr_as(&args[0], &XeType::Unknown);
            let item = self.expr_as(&args[1], &XeType::Unknown);
            return format!("xe_builtin_append({}, {})", list, item);
        };

        let container = match self.place_types(&place) {
            Ok(types) => types.last().cloned().unwrap_or(XeType::Unknown),
            Err(message) => return runtime_error(&message),
        };
        let item_ty = match &container {
            XeType::List(inner) => (**inner).clone(),
            _ => XeType::Unknown,
        };
        let item = self.expr_as(&args[1], &item_ty);
        self.place_op(
            &place,
            vec![format!("let __xe_item = {};", item)],
            Vec::new(),
            |c, ty| {
                let (push, list_value) = match ty {
                    XeType::List(_) => ("__xe_t.push(__xe_item);", "XeValue::from(__xe_t.clone())"),
                    ty if ty.is_dynamic() => ("xe_val_push(__xe_t, __xe_item);", "__xe_t.clone()"),
                    ty => return runtime_error(&format!("append() expected list, got {}", ty)),
                };
                if want_value {
                    format!("{{ let __xe_t = {}; {} {} }}", c, push, list_value)
                } else {
                    format!("{{ let __xe_t = {}; {} }}", c, push)
                }
            },
        )
    }

    /// `pop(list)` removes and returns the last element of the list in place.
    fn generate_pop(&mut self, args: &[TypedExpression]) -> String {
        let Some(place) = self.resolve_place(strip_coercions(&args[0])) else {
            let list = self.expr_as(&args[0], &XeType::Unknown);
            return format!("xe_builtin_pop({})", list);
        };
        self.place_op(&place, Vec::new(), Vec::new(), |c, ty| match ty {
            XeType::List(inner) => convert(&format!("{}.xe_pop()", c), inner, &XeType::Unknown),
            ty if ty.is_dynamic() => format!("xe_val_pop({})", c),
            ty => runtime_error(&format!("pop() expected list, got {}", ty)),
        })
    }

    fn resolve_place<'a>(&self, expr: &'a TypedExpression) -> Option<Place<'a>> {
        match &expr.kind {
            TypedExpressionKind::Identifier(name) => {
                let (root, root_ty) = if let Some(ty) = self.local_type(name) {
                    (PlaceRoot::Local(name.clone()), ty)
                } else {
                    let ty = self.globals.get(name)?.clone();
                    (PlaceRoot::Global(name.clone()), ty)
                };
                Some(Place {
                    root,
                    root_ty,
                    steps: Vec::new(),
                })
            }
            TypedExpressionKind::Index { object, index } => {
                let mut place = self.resolve_place(object)?;
                place.steps.push(PlaceStep::Index(index));
                Some(place)
            }
            TypedExpressionKind::FieldAccess { object, field } => {
                let mut place = self.resolve_place(object)?;
                place.steps.push(PlaceStep::Field(field));
                Some(place)
            }
            _ => None,
        }
    }

    /// The storage type at the root and after each step of a place.
    fn place_types(&self, place: &Place) -> Result<Vec<XeType>, String> {
        let mut types = vec![place.root_ty.clone()];
        for step in &place.steps {
            let current = types.last().unwrap();
            let next = match (step, current) {
                (PlaceStep::Index(_), XeType::List(inner)) => (**inner).clone(),
                (_, ty) if ty.is_dynamic() => XeType::Unknown,
                (PlaceStep::Index(_), ty) => return Err(format!("cannot index into {} for assignment", ty)),
                (PlaceStep::Field(field), ty) => {
                    return Err(format!("cannot access field '{}' on {}", field, ty))
                }
            };
            types.push(next);
        }
        Ok(types)
    }

    /// Runs `op` on a mutable reference to the storage of `place`. `before` is evaluated
    /// first, then the index expressions of the place, then `after`, and only then is
    /// the storage borrowed, so user code never runs while the borrow is held.
    fn place_op(
        &mut self,
        place: &Place,
        before: Vec<String>,
        after: Vec<String>,
        op: impl FnOnce(&str, &XeType) -> String,
    ) -> String {
        let types = match self.place_types(place) {
            Ok(types) => types,
            Err(message) => return format!("{{ {} {} }}", before.join(" "), runtime_error(&message)),
        };

        let mut lets = before;
        let mut target = "__xe_root".to_string();
        for (i, step) in place.steps.iter().enumerate() {
            match (step, &types[i]) {
                (PlaceStep::Index(index), XeType::List(_)) => {
                    let key = self.expr_as(index, &XeType::Number);
                    lets.push(format!("let __xe_k{} = {};", i, key));
                    target = format!("xe_vec_slot({}, __xe_k{})", target, i);
                }
                (PlaceStep::Index(index), _) => {
                    let key = self.expr_as(index, &XeType::Unknown);
                    lets.push(format!("let __xe_k{} = {};", i, key));
                    target = format!("xe_dyn_slot({}, &__xe_k{})", target, i);
                }
                (PlaceStep::Field(field), _) => {
                    target = format!("xe_field_slot({}, {:?})", target, field);
                }
            }
        }
        lets.extend(after);

        let body = op(&target, types.last().unwrap());
        match &place.root {
            PlaceRoot::Local(name) => format!(
                "{{ {} let __xe_root = &mut {}; {} }}",
                lets.join(" "),
                local_name(name),
                body
            ),
            PlaceRoot::Global(name) => format!(
                "{{ {} xe_gmut(&{}, {:?}, move |__xe_root| {{ {} }}) }}",
                lets.join(" "),
                global_key(name),
                display_name(name),
                body
            ),
        }
    }

    fn local_type(&self, name: &str) -> Option<XeType> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name)).cloned()
    }

    fn define_local(&mut self, name: &str, ty: XeType) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), ty);
        }
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

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts Rust code producing a value of type `from` into code producing type `to`.
/// Conversions out of dynamic values are checked at runtime.
fn convert(code: &str, from: &XeType, to: &XeType) -> String {
    if from == to {
        return code.to_string();
    }
    if *from == XeType::Void {
        return convert(&format!("{{ {}; XeValue::None }}", code), &XeType::Unknown, to);
    }
    if *to == XeType::Void {
        return format!("{{ let _ = {}; }}", code);
    }
    match (from.is_dynamic(), to.is_dynamic()) {
        (true, true) => code.to_string(),
        (false, true) => format!("XeValue::from({})", code),
        (true, false) => from_dynamic(code, to),
        (false, false) => match (from, to) {
            (XeType::List(a), XeType::List(b)) => format!(
                "({}).into_iter().map(|__v| {}).collect::<Vec<_>>()",
                code,
                convert("__v", a, b)
            ),
            _ => from_dynamic(&format!("XeValue::from({})", code), to),
        },
    }
}

fn from_dynamic(code: &str, to: &XeType) -> String {
    match to {
        XeType::Number => format!("({}).as_f64()", code),
        XeType::Boolean => format!("({}).as_bool()", code),
        XeType::Text => format!("({}).as_string()", code),
        XeType::List(inner) => format!(
            "({}).as_list().into_iter().map(|__v| {}).collect::<Vec<_>>()",
            code,
            convert("__v", &XeType::Unknown, inner)
        ),
        _ => code.to_string(),
    }
}

fn strip_coercions(expr: &TypedExpression) -> &TypedExpression {
    match &expr.kind {
        TypedExpressionKind::Wrap(inner) | TypedExpressionKind::Unwrap(inner, _) => {
            strip_coercions(inner)
        }
        _ => expr,
    }
}

fn runtime_error(message: &str) -> String {
    format!("xe_runtime_error({:?})", message)
}

/// Locals get a prefix so they can never collide with Rust keywords or runtime names.
fn local_name(name: &str) -> String {
    format!("l_{}", name)
}

fn global_key(name: &str) -> String {
    format!("XE_G_{}", name)
}

fn function_name(name: &str) -> String {
    if name.starts_with("xe_m") {
        name.to_string()
    } else {
        format!("f_{}", name)
    }
}

/// The user-facing name of a linked symbol such as `xe_m0_count`.
fn display_name(name: &str) -> &str {
    name.strip_prefix("xe_m")
        .map(|rest| rest.trim_start_matches(|c: char| c.is_ascii_digit()))
        .and_then(|rest| rest.strip_prefix('_'))
        .unwrap_or(name)
}

const PRELUDE: &str = r#"#![allow(dead_code, unused_mut, unused_variables, non_snake_case, non_upper_case_globals, unused_parens, unused_braces, unreachable_code)]
use std::io::{self, Write};

#[derive(Clone, Debug)]
enum XeValue {
    Number(f64),
    Text(String),
    Boolean(bool),
    List(Vec<XeValue>),
    Map(std::collections::HashMap<String, XeValue>),
    Struct {
        name: String,
        fields: std::collections::HashMap<String, XeValue>,
    },
    None,
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

impl From<XeValue> for Vec<String> {
    fn from(v: XeValue) -> Self {
        v.as_list().into_iter().map(|item| item.to_string()).collect()
    }
}

impl From<XeValue> for Vec<f64> {
    fn from(v: XeValue) -> Self {
        v.as_list().into_iter().map(|item| item.as_f64()).collect()
    }
}

impl From<XeValue> for Vec<bool> {
    fn from(v: XeValue) -> Self {
        v.as_list().into_iter().map(|item| item.as_bool()).collect()
    }
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
                if *n == 0.0 {
                    // Avoid printing negative zero as "-0".
                    write!(f, "0")
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
            XeValue::Map(m) => {
                write!(f, "{{")?;
                let mut entries = m.iter().collect::<Vec<_>>();
                entries.sort_by_key(|(k, _)| (*k).clone());
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "\"{}\": {}", k, v)?;
                }
                write!(f, "}}")
            }
            XeValue::Struct { name, fields } => {
                write!(f, "{} {{ ", name)?;
                let mut entries = fields.iter().collect::<Vec<_>>();
                entries.sort_by_key(|(k, _)| (*k).clone());
                for (i, (k, v)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, " }}")
            }
            XeValue::None => write!(f, "none"),
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
            XeValue::Map(m) => !m.is_empty(),
            XeValue::Struct { .. } => true,
            XeValue::None => false,
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
            XeValue::Map(_) => "map",
            XeValue::Struct { .. } => "struct",
            XeValue::None => "none",
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
        XeValue::Text(s) => s.chars().count() as f64,
        XeValue::List(l) => l.len() as f64,
        XeValue::Map(m) => m.len() as f64,
        XeValue::Struct { fields, .. } => fields.len() as f64,
        _ => xe_runtime_error(&format!(
            "length() expected text, list, map, or struct, got {}",
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
            _ => xe_runtime_error("cannot convert value to number"),
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
        (XeValue::None, XeValue::None) => true,
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
        (XeValue::Map(a), XeValue::Map(b)) => {
            if a.len() != b.len() {
                return false;
            }
            for (k, v) in a {
                if let Some(bv) = b.get(k) {
                    if !xe_eq(v, bv) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        }
        (XeValue::Struct { name: n1, fields: f1 }, XeValue::Struct { name: n2, fields: f2 }) => {
            if n1 != n2 || f1.len() != f2.len() {
                return false;
            }
            for (k, v) in f1 {
                if let Some(bv) = f2.get(k) {
                    if !xe_eq(v, bv) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
            true
        }
        _ => false,
    }
}

fn xe_index_check(idx: f64) -> usize {
    if !idx.is_finite() || idx < 0.0 || idx.fract() != 0.0 {
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

fn xe_index(obj: &XeValue, idx: &XeValue) -> XeValue {
    match obj {
        XeValue::List(l) => {
            let i = xe_index_check(idx.as_f64());
            l.get(i).cloned().unwrap_or_else(|| xe_runtime_error(&format!("list index {} out of bounds", i)))
        }
        XeValue::Text(s) => {
            let i = xe_index_check(idx.as_f64());
            s.chars().nth(i).map(|c| XeValue::Text(c.to_string())).unwrap_or_else(|| {
                xe_runtime_error(&format!("text index {} out of bounds", i))
            })
        }
        XeValue::Map(m) => {
            let k = idx.to_string();
            m.get(&k).cloned().unwrap_or_else(|| {
                xe_runtime_error(&format!("key '{}' not found in map", k))
            })
        }
        XeValue::Struct { name, fields } => {
            let k = idx.to_string();
            fields.get(&k).cloned().unwrap_or_else(|| {
                xe_runtime_error(&format!("struct '{}' has no field '{}'", name, k))
            })
        }
        _ => xe_runtime_error(&format!(
            "index access expected text, list, map, or struct, got {}",
            obj.type_name()
        )),
    }
}

fn xe_set_index(obj: &mut XeValue, idx: &XeValue, val: XeValue) {
    match obj {
        XeValue::List(l) => {
            let i = xe_index_check(idx.as_f64());
            if i >= l.len() {
                xe_runtime_error(&format!("list index {} out of bounds", i));
            }
            l[i] = val;
        }
        XeValue::Map(m) => {
            m.insert(idx.to_string(), val);
        }
        XeValue::Struct { fields, .. } => {
            fields.insert(idx.to_string(), val);
        }
        _ => xe_runtime_error(&format!("cannot index assign to {}", obj.type_name())),
    }
}

fn xe_vec_set_index<T>(list: &mut Vec<T>, idx: f64, val: T) {
    let i = xe_index_check(idx);
    if i >= list.len() {
        xe_runtime_error(&format!("list index {} out of bounds", i));
    }
    list[i] = val;
}

fn xe_get_field(obj: &XeValue, field: &str) -> XeValue {
    match obj {
        XeValue::Struct { name, fields } => {
            fields.get(field).cloned().unwrap_or_else(|| {
                xe_runtime_error(&format!("struct '{}' has no field '{}'", name, field))
            })
        }
        XeValue::Map(m) => {
            m.get(field).cloned().unwrap_or_else(|| {
                xe_runtime_error(&format!("map has no field '{}'", field))
            })
        }
        _ => xe_runtime_error(&format!("cannot access field '{}' on {}", field, obj.type_name())),
    }
}

fn xe_set_field(obj: &mut XeValue, field: &str, val: XeValue) {
    match obj {
        XeValue::Struct { fields, .. } => {
            fields.insert(field.to_string(), val);
        }
        XeValue::Map(m) => {
            m.insert(field.to_string(), val);
        }
        _ => xe_runtime_error(&format!("cannot set field '{}' on {}", field, obj.type_name())),
    }
}

fn xe_make_map(pairs: Vec<(String, XeValue)>) -> XeValue {
    let mut map = std::collections::HashMap::new();
    for (k, v) in pairs {
        map.insert(k, v);
    }
    XeValue::Map(map)
}

fn xe_make_struct(name: &str, field_names: &[&str], values: Vec<XeValue>) -> XeValue {
    let mut fields = std::collections::HashMap::new();
    for (k, v) in field_names.iter().zip(values.into_iter()) {
        fields.insert(k.to_string(), v);
    }
    XeValue::Struct {
        name: name.to_string(),
        fields,
    }
}

fn xe_val_push(val: &mut XeValue, item: XeValue) {
    match val {
        XeValue::List(l) => l.push(item),
        _ => xe_runtime_error(&format!("append() expected list, got {}", val.type_name())),
    }
}

fn xe_val_pop(val: &mut XeValue) -> XeValue {
    match val {
        XeValue::List(l) => l.pop().unwrap_or_else(|| xe_runtime_error("pop() called on empty list")),
        _ => xe_runtime_error(&format!("pop() expected list, got {}", val.type_name())),
    }
}

fn xe_builtin_append(mut target: XeValue, item: XeValue) -> XeValue {
    xe_val_push(&mut target, item);
    target
}

fn xe_builtin_pop(mut target: XeValue) -> XeValue {
    xe_val_pop(&mut target)
}

trait XePush<T> {
    fn xe_push(&mut self, item: T);
}

impl<T> XePush<T> for Vec<T> {
    fn xe_push(&mut self, item: T) {
        self.push(item);
    }
}

impl XePush<XeValue> for XeValue {
    fn xe_push(&mut self, item: XeValue) {
        xe_val_push(self, item);
    }
}

trait XePop {
    type Item;
    fn xe_pop(&mut self) -> Self::Item;
}

impl<T> XePop for Vec<T> {
    type Item = T;
    fn xe_pop(&mut self) -> Self::Item {
        self.pop().unwrap_or_else(|| xe_runtime_error("pop() called on empty list"))
    }
}

impl XePop for XeValue {
    type Item = XeValue;
    fn xe_pop(&mut self) -> Self::Item {
        xe_val_pop(self)
    }
}

fn xe_builtin_keys(target: &XeValue) -> Vec<String> {
    match target {
        XeValue::Map(m) => {
            let mut keys = m.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            keys
        }
        XeValue::Struct { fields, .. } => {
            let mut keys = fields.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            keys
        }
        _ => xe_runtime_error(&format!("keys() expected map or struct, got {}", target.type_name())),
    }
}

fn xe_builtin_values(target: &XeValue) -> Vec<XeValue> {
    match target {
        XeValue::Map(m) => {
            let mut pairs = m.iter().collect::<Vec<_>>();
            pairs.sort_by_key(|(k, _)| (*k).clone());
            pairs.into_iter().map(|(_, v)| v.clone()).collect()
        }
        XeValue::Struct { fields, .. } => {
            let mut pairs = fields.iter().collect::<Vec<_>>();
            pairs.sort_by_key(|(k, _)| (*k).clone());
            pairs.into_iter().map(|(_, v)| v.clone()).collect()
        }
        _ => xe_runtime_error(&format!("values() expected map or struct, got {}", target.type_name())),
    }
}

fn xe_builtin_has_key(target: &XeValue, key: &XeValue) -> bool {
    match target {
        XeValue::Map(m) => m.contains_key(&key.to_string()),
        XeValue::Struct { fields, .. } => fields.contains_key(&key.to_string()),
        _ => false,
    }
}

fn xe_builtin_contains(target: &XeValue, item: &XeValue) -> bool {
    match target {
        XeValue::List(l) => l.iter().any(|v| xe_eq(v, item)),
        XeValue::Text(s) => s.contains(&item.to_string()),
        XeValue::Map(m) => m.contains_key(&item.to_string()),
        _ => false,
    }
}

fn xe_builtin_split(text: &str, delim: &str) -> Vec<String> {
    text.split(delim).map(|part| part.to_string()).collect()
}

fn xe_builtin_join(list: &[XeValue], delim: &str) -> String {
    list.iter().map(|item| item.to_string()).collect::<Vec<_>>().join(delim)
}

fn xe_iter(value: &XeValue) -> Vec<XeValue> {
    match value {
        XeValue::List(items) => items.clone(),
        XeValue::Text(s) => s.chars().map(|c| XeValue::Text(c.to_string())).collect(),
        XeValue::Map(m) => {
            let mut keys = m.keys().cloned().map(XeValue::Text).collect::<Vec<_>>();
            keys.sort_by_key(|k| k.to_string());
            keys
        }
        XeValue::Struct { fields, .. } => {
            let mut keys = fields.keys().cloned().map(XeValue::Text).collect::<Vec<_>>();
            keys.sort_by_key(|k| k.to_string());
            keys
        }
        _ => xe_runtime_error(&format!(
            "for-loop iteration expected text, list, map, or struct, got {}",
            value.type_name()
        )),
    }
}

type XeGlobal<T> = std::thread::LocalKey<std::cell::RefCell<Option<T>>>;

fn xe_unassigned(name: &str) -> ! {
    xe_runtime_error(&format!("variable '{}' was used before it was assigned", name))
}

fn xe_gget<T: Clone + 'static>(key: &'static XeGlobal<T>, name: &str) -> T {
    key.with(|cell| match &*cell.borrow() {
        Some(value) => value.clone(),
        None => xe_unassigned(name),
    })
}

fn xe_gwith<T: 'static, R>(key: &'static XeGlobal<T>, name: &str, f: impl FnOnce(&T) -> R) -> R {
    key.with(|cell| match &*cell.borrow() {
        Some(value) => f(value),
        None => xe_unassigned(name),
    })
}

fn xe_gset<T: 'static>(key: &'static XeGlobal<T>, value: T) {
    key.with(|cell| *cell.borrow_mut() = Some(value));
}

fn xe_gmut<T: 'static, R>(key: &'static XeGlobal<T>, name: &str, f: impl FnOnce(&mut T) -> R) -> R {
    key.with(|cell| match &mut *cell.borrow_mut() {
        Some(value) => f(value),
        None => xe_unassigned(name),
    })
}

fn xe_vec_slot<T>(list: &mut Vec<T>, idx: f64) -> &mut T {
    let i = xe_index_check(idx);
    let len = list.len();
    if i >= len {
        xe_runtime_error(&format!("list index {} out of bounds", i));
    }
    &mut list[i]
}

fn xe_dyn_slot<'a>(obj: &'a mut XeValue, key: &XeValue) -> &'a mut XeValue {
    match obj {
        XeValue::List(l) => {
            let i = xe_index_check(key.as_f64());
            let len = l.len();
            if i >= len {
                xe_runtime_error(&format!("list index {} out of bounds", i));
            }
            &mut l[i]
        }
        XeValue::Map(m) => {
            let k = key.to_string();
            match m.get_mut(&k) {
                Some(value) => value,
                None => xe_runtime_error(&format!("key '{}' not found in map", k)),
            }
        }
        XeValue::Struct { name, fields } => {
            let k = key.to_string();
            match fields.get_mut(&k) {
                Some(value) => value,
                None => xe_runtime_error(&format!("struct '{}' has no field '{}'", name, k)),
            }
        }
        other => xe_runtime_error(&format!(
            "index access expected list, map, or struct, got {}",
            other.type_name()
        )),
    }
}

fn xe_field_slot<'a>(obj: &'a mut XeValue, field: &str) -> &'a mut XeValue {
    match obj {
        XeValue::Struct { name, fields } => match fields.get_mut(field) {
            Some(value) => value,
            None => xe_runtime_error(&format!("struct '{}' has no field '{}'", name, field)),
        },
        XeValue::Map(m) => match m.get_mut(field) {
            Some(value) => value,
            None => xe_runtime_error(&format!("map has no field '{}'", field)),
        },
        other => xe_runtime_error(&format!(
            "cannot access field '{}' on {}",
            field,
            other.type_name()
        )),
    }
}

"#;
