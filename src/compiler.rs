use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::*;
use crate::codegen::CodeGenerator;
use crate::error::{Span, XeError, XeErrorKind};
use crate::lexer::Lexer;
use crate::parser::Parser;

pub struct CompilationFailure {
    pub error: XeError,
    pub source: String,
}

#[derive(Clone)]
struct ResolvedImport {
    module_id: usize,
    kind: ResolvedImportKind,
    span: Span,
}

#[derive(Clone)]
enum ResolvedImportKind {
    /// `import a.b` (binding "a.b") or `import a.b as c` (binding "c").
    Module(String),
    /// `from m import x as y`: (imported name, local name)
    Names(Vec<(String, String)>),
}

struct ModuleRecord {
    id: usize,
    path: PathBuf,
    program: Program,
    /// Every module-level name (functions, structs and variables) and its linked symbol.
    exports: HashMap<String, String>,
    imports: Vec<ResolvedImport>,
}

/// The names visible in a module through its imports.
struct ImportScope {
    /// `from m import x as y`: local name -> linked symbol
    names: HashMap<String, String>,
    /// `import m` bindings: binding path ("m", "a.b" or an alias) -> module id
    modules: HashMap<String, usize>,
}

pub fn compile_path(entry_path: &Path) -> Result<String, Box<CompilationFailure>> {
    let mut compiler = ModuleCompiler::new();
    let entry_id = compiler
        .load_entry_module(entry_path)
        .map_err(|error| Box::new(compiler.failure_for_error(error)))?;

    let linked_program = compiler
        .link_program(entry_id)
        .map_err(|error| Box::new(compiler.failure_for_error(error)))?;

    let typed_program = crate::semantic::analyze_program(&linked_program)
        .map_err(|error| Box::new(compiler.failure_for_error(error)))?;

    let mut codegen = CodeGenerator::new(compiler.sources.clone());
    Ok(codegen.generate(&typed_program))
}

struct ModuleCompiler {
    next_module_id: usize,
    modules: HashMap<usize, ModuleRecord>,
    module_ids_by_path: HashMap<PathBuf, usize>,
    loading_stack: Vec<PathBuf>,
    sources: HashMap<String, String>,
    /// Linked symbols of every module-level variable (as opposed to functions and structs).
    variable_symbols: HashSet<String>,
}

/// Everything name resolution needs while rewriting one module.
struct Resolver<'a> {
    module: &'a ModuleRecord,
    imports: &'a ImportScope,
}

impl ModuleCompiler {
    fn new() -> Self {
        Self {
            next_module_id: 0,
            modules: HashMap::new(),
            module_ids_by_path: HashMap::new(),
            loading_stack: Vec::new(),
            sources: HashMap::new(),
            variable_symbols: HashSet::new(),
        }
    }

    fn load_entry_module(&mut self, entry_path: &Path) -> Result<usize, XeError> {
        let canonical = fs::canonicalize(entry_path).map_err(|error| {
            XeError::new(
                XeErrorKind::IoError(format!("{}: {}", entry_path.display(), error)),
                None,
            )
        })?;

        self.load_module(&canonical, None)
    }

    fn load_module(&mut self, path: &Path, import_span: Option<&Span>) -> Result<usize, XeError> {
        let canonical = fs::canonicalize(path)
            .map_err(|error| XeError::new(XeErrorKind::IoError(error.to_string()), None))?;

        if let Some(module_id) = self.module_ids_by_path.get(&canonical) {
            return Ok(*module_id);
        }

        if let Some(position) = self
            .loading_stack
            .iter()
            .position(|current| current == &canonical)
        {
            let cycle = self.loading_stack[position..]
                .iter()
                .chain(std::iter::once(&canonical))
                .map(|item| item.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            return Err(XeError::new(
                XeErrorKind::CircularImport(cycle),
                import_span.cloned(),
            ));
        }

        let source_name = canonical.display().to_string();
        let source = fs::read_to_string(&canonical).map_err(|error| {
            XeError::new(
                XeErrorKind::IoError(format!("{}: {}", source_name, error)),
                None,
            )
        })?;
        // Recorded before parsing so syntax errors can show the offending line.
        self.sources.insert(source_name.clone(), source.clone());

        self.loading_stack.push(canonical.clone());

        let result = (|| -> Result<ModuleRecord, XeError> {
            let program = self.parse_program(&source, &source_name)?;
            validate_import_placement(&program)?;
            for statement in &program.statements {
                validate_nesting(statement, true)?;
            }

            let module_id = self.allocate_module_id();
            let (exports, variables) = collect_exports(module_id, &program)?;
            for variable in &variables {
                self.variable_symbols.insert(exports[variable].clone());
            }
            let imports = self.resolve_imports(&canonical, &program)?;

            Ok(ModuleRecord {
                id: module_id,
                path: path.to_path_buf(),
                program,
                exports,
                imports,
            })
        })();

        self.loading_stack.pop();

        let module = result?;
        self.module_ids_by_path.insert(module.path.clone(), module.id);
        self.modules.insert(module.id, module);

        Ok(*self.module_ids_by_path.get(&canonical).unwrap())
    }

    fn parse_program(&self, source: &str, source_name: &str) -> Result<Program, XeError> {
        let mut lexer = Lexer::new_with_source(source, Some(source_name.to_string()));
        let tokens = lexer.tokenize()?;

        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    fn resolve_imports(
        &mut self,
        module_path: &Path,
        program: &Program,
    ) -> Result<Vec<ResolvedImport>, XeError> {
        let module_dir = module_path.parent().unwrap_or_else(|| Path::new("."));
        let mut imports = Vec::new();

        for statement in &program.statements {
            let (module, kind) = match &statement.kind {
                StatementKind::Import { module, alias } => (
                    module,
                    ResolvedImportKind::Module(alias.clone().unwrap_or_else(|| module.as_string())),
                ),
                StatementKind::FromImport { module, names } => {
                    (module, ResolvedImportKind::Names(names.clone()))
                }
                _ => continue,
            };
            let dependency_path = self.resolve_module_path(module_dir, module, &statement.span)?;
            let dependency_id = self.load_module(&dependency_path, Some(&statement.span))?;
            imports.push(ResolvedImport {
                module_id: dependency_id,
                kind,
                span: statement.span.clone(),
            });
        }

        Ok(imports)
    }

    fn resolve_module_path(
        &self,
        base_dir: &Path,
        module: &ModulePath,
        span: &Span,
    ) -> Result<PathBuf, XeError> {
        let mut base_path = base_dir.to_path_buf();
        for segment in &module.segments {
            base_path.push(segment);
        }

        let file_candidate = base_path.with_extension("xe");
        if file_candidate.is_file() {
            return Ok(file_candidate);
        }

        let index_candidate = base_path.join("index.xe");
        if index_candidate.is_file() {
            return Ok(index_candidate);
        }

        Err(XeError::new(
            XeErrorKind::ModuleNotFound(module.as_string()),
            Some(span.clone()),
        ))
    }

    fn link_program(&self, entry_id: usize) -> Result<Program, XeError> {
        let mut statements = Vec::new();
        let mut module_ids = self.modules.keys().copied().collect::<Vec<_>>();
        module_ids.sort_unstable();

        // 1. Collect all function and struct definitions from all modules
        for module_id in &module_ids {
            let module = &self.modules[module_id];
            let imports = self.build_import_scope(module)?;
            let resolver = Resolver {
                module,
                imports: &imports,
            };

            for statement in &module.program.statements {
                let kind = match &statement.kind {
                    StatementKind::FunctionDef { .. } => self.rewrite_function_def(statement, &resolver)?,
                    StatementKind::StructDef { name, fields } => StatementKind::StructDef {
                        name: module.exports[name].clone(),
                        fields: fields.clone(),
                    },
                    _ => continue,
                };
                statements.push(Statement {
                    kind,
                    span: statement.span.clone(),
                });
            }
        }

        // 2. Top-level code of every module in dependency order, the entry module last
        let mut order = self.initialization_order(entry_id);
        order.push(entry_id);
        for module_id in order {
            let module = &self.modules[&module_id];
            let imports = self.build_import_scope(module)?;
            let resolver = Resolver {
                module,
                imports: &imports,
            };
            let mut scopes = vec![HashSet::new()];
            for statement in &module.program.statements {
                if matches!(
                    statement.kind,
                    StatementKind::Import { .. }
                        | StatementKind::FromImport { .. }
                        | StatementKind::FunctionDef { .. }
                        | StatementKind::StructDef { .. }
                ) {
                    continue;
                }
                statements.push(self.rewrite_statement(statement, &resolver, &mut scopes, None)?);
            }
        }

        Ok(Program { statements })
    }

    fn build_import_scope(&self, module: &ModuleRecord) -> Result<ImportScope, XeError> {
        let mut scope = ImportScope {
            names: HashMap::new(),
            modules: HashMap::new(),
        };

        for import in &module.imports {
            let dependency = &self.modules[&import.module_id];
            match &import.kind {
                ResolvedImportKind::Module(binding) => {
                    let root = binding.split('.').next().unwrap_or(binding);
                    if module.exports.contains_key(root) {
                        return Err(XeError::new(
                            XeErrorKind::ImportNameConflict(root.to_string()),
                            Some(import.span.clone()),
                        ));
                    }
                    if scope.modules.insert(binding.clone(), import.module_id).is_some() {
                        return Err(XeError::new(
                            XeErrorKind::DuplicateImport(binding.clone()),
                            Some(import.span.clone()),
                        ));
                    }
                }
                ResolvedImportKind::Names(names) => {
                    for (name, local) in names {
                        let Some(symbol) = dependency.exports.get(name) else {
                            return Err(XeError::new(
                                XeErrorKind::ImportedNameNotFound {
                                    module: dependency.path.display().to_string(),
                                    name: name.clone(),
                                },
                                Some(import.span.clone()),
                            ));
                        };
                        if module.exports.contains_key(local) {
                            return Err(XeError::new(
                                XeErrorKind::ImportNameConflict(local.clone()),
                                Some(import.span.clone()),
                            ));
                        }
                        if scope.names.insert(local.clone(), symbol.clone()).is_some() {
                            return Err(XeError::new(
                                XeErrorKind::DuplicateImport(local.clone()),
                                Some(import.span.clone()),
                            ));
                        }
                    }
                }
            }
        }

        for binding in scope.modules.keys() {
            let root = binding.split('.').next().unwrap_or(binding);
            if scope.names.contains_key(root) {
                return Err(XeError::new(
                    XeErrorKind::ImportNameConflict(root.to_string()),
                    None,
                ));
            }
        }

        Ok(scope)
    }

    fn rewrite_statement_block(
        &self,
        statements: &[Statement],
        resolver: &Resolver,
        scopes: &mut Vec<HashSet<String>>,
        ctx: Option<&FunctionContext>,
    ) -> Result<Vec<Statement>, XeError> {
        statements
            .iter()
            .map(|statement| self.rewrite_statement(statement, resolver, scopes, ctx))
            .collect()
    }

    /// Resolves a module-level name (own export or `from` import) to its linked symbol.
    fn module_symbol(&self, name: &str, resolver: &Resolver) -> Option<String> {
        resolver
            .module
            .exports
            .get(name)
            .or_else(|| resolver.imports.names.get(name))
            .cloned()
    }

    fn rewrite_function_def(
        &self,
        statement: &Statement,
        resolver: &Resolver,
    ) -> Result<StatementKind, XeError> {
        let StatementKind::FunctionDef { name, params, body } = &statement.kind else {
            unreachable!("rewrite_function_def called on a non-function statement");
        };

        let ctx = FunctionContext::collect(body);
        for global in &ctx.globals {
            if params.contains(global) {
                return Err(XeError::new(
                    XeErrorKind::InvalidGlobal(format!(
                        "'{}' is a parameter of '{}' and cannot also be declared global",
                        global, name
                    )),
                    Some(statement.span.clone()),
                ));
            }
        }

        let mut scopes = vec![HashSet::new(), params.iter().cloned().collect()];
        let rewritten_body = self.rewrite_statement_block(body, resolver, &mut scopes, Some(&ctx))?;

        Ok(StatementKind::FunctionDef {
            name: resolver.module.exports[name].clone(),
            params: params.clone(),
            body: rewritten_body,
        })
    }

    /// The linked name for an assignment target or catch variable.
    fn resolve_target(
        &self,
        name: &str,
        resolver: &Resolver,
        scopes: &mut [HashSet<String>],
        ctx: Option<&FunctionContext>,
    ) -> String {
        match ctx {
            Some(ctx) if ctx.globals.contains(name) => self
                .module_symbol(name, resolver)
                .unwrap_or_else(|| name.to_string()),
            Some(_) => {
                if !is_in_local_scope(scopes, name) {
                    scopes.last_mut().unwrap().insert(name.to_string());
                }
                name.to_string()
            }
            None => {
                if is_in_local_scope(scopes, name) {
                    name.to_string()
                } else {
                    self.module_symbol(name, resolver)
                        .unwrap_or_else(|| name.to_string())
                }
            }
        }
    }

    /// The linked name for a read of `name` (a variable, a function, or a builtin).
    fn resolve_read(
        &self,
        name: &str,
        span: &Span,
        resolver: &Resolver,
        scopes: &[HashSet<String>],
        ctx: Option<&FunctionContext>,
    ) -> Result<String, XeError> {
        if is_in_local_scope(scopes, name) {
            return Ok(name.to_string());
        }
        if let Some(ctx) = ctx {
            if ctx.globals.contains(name) {
                return Ok(self
                    .module_symbol(name, resolver)
                    .unwrap_or_else(|| name.to_string()));
            }
            if ctx.assigned.contains(name) {
                // Assigned somewhere in this function, so the name is local everywhere in it.
                let is_module_variable = self
                    .module_symbol(name, resolver)
                    .is_some_and(|symbol| self.variable_symbols.contains(&symbol));
                if is_module_variable {
                    return Err(XeError::new(
                        XeErrorKind::LocalUsedBeforeAssignment(name.to_string()),
                        Some(span.clone()),
                    ));
                }
                return Ok(name.to_string());
            }
        }
        if let Some(symbol) = self.module_symbol(name, resolver) {
            return Ok(symbol);
        }
        if resolver
            .imports
            .modules
            .keys()
            .any(|binding| binding.split('.').next() == Some(name))
        {
            return Err(XeError::new(
                XeErrorKind::ModuleUsedAsValue(name.to_string()),
                Some(span.clone()),
            ));
        }
        Ok(name.to_string())
    }

    /// The linked name for `object.method(...)` called as a function: a function of
    /// this module, an imported function, or a builtin. Variables are not methods.
    fn resolve_method(&self, method: &str, resolver: &Resolver) -> String {
        match self.module_symbol(method, resolver) {
            Some(symbol) if !self.variable_symbols.contains(&symbol) => symbol,
            _ => method.to_string(),
        }
    }

    /// If `expression` is `module.member` (possibly followed by more fields), returns the
    /// module id, the member name and the remaining field names.
    fn split_module_access<'e>(
        &self,
        expression: &'e Expression,
        resolver: &Resolver,
        scopes: &[HashSet<String>],
    ) -> Option<(usize, Vec<&'e str>)> {
        let mut chain = Vec::new();
        let mut current = expression;
        loop {
            match &current.kind {
                ExpressionKind::FieldAccess { object, field } => {
                    chain.push(field.as_str());
                    current = object;
                }
                ExpressionKind::Identifier(root) => {
                    chain.push(root.as_str());
                    break;
                }
                _ => return None,
            }
        }
        chain.reverse();

        let root = chain[0];
        if is_in_local_scope(scopes, root) || self.module_symbol(root, resolver).is_some() {
            return None;
        }
        // Longest binding first, so `import a.b` wins over `import a` for `a.b.x`.
        (1..=chain.len()).rev().find_map(|length| {
            let binding = chain[..length].join(".");
            resolver
                .imports
                .modules
                .get(&binding)
                .map(|module_id| (*module_id, chain[length..].to_vec()))
        })
    }

    fn module_member_symbol(
        &self,
        module_id: usize,
        member: &str,
        span: &Span,
    ) -> Result<String, XeError> {
        let dependency = &self.modules[&module_id];
        dependency.exports.get(member).cloned().ok_or_else(|| {
            XeError::new(
                XeErrorKind::ImportedNameNotFound {
                    module: dependency.path.display().to_string(),
                    name: member.to_string(),
                },
                Some(span.clone()),
            )
        })
    }

    /// Rewrites `module.member.rest...` into the member's symbol followed by field accesses.
    fn rewrite_module_access(
        &self,
        expression: &Expression,
        resolver: &Resolver,
        scopes: &[HashSet<String>],
    ) -> Result<Option<Expression>, XeError> {
        let Some((module_id, rest)) = self.split_module_access(expression, resolver, scopes) else {
            return Ok(None);
        };
        let span = &expression.span;
        let Some((member, fields)) = rest.split_first() else {
            return Err(XeError::new(
                XeErrorKind::ModuleUsedAsValue(expression_path(expression)),
                Some(span.clone()),
            ));
        };
        let mut result = Expression {
            kind: ExpressionKind::Identifier(self.module_member_symbol(module_id, member, span)?),
            span: span.clone(),
        };
        for field in fields {
            result = Expression {
                kind: ExpressionKind::FieldAccess {
                    object: Box::new(result),
                    field: field.to_string(),
                },
                span: span.clone(),
            };
        }
        Ok(Some(result))
    }

    fn rewrite_statement(
        &self,
        statement: &Statement,
        resolver: &Resolver,
        scopes: &mut Vec<HashSet<String>>,
        ctx: Option<&FunctionContext>,
    ) -> Result<Statement, XeError> {
        let kind = match &statement.kind {
            StatementKind::Import { .. } | StatementKind::FromImport { .. } => {
                statement.kind.clone()
            }
            StatementKind::Global { names } => {
                if ctx.is_none() {
                    return Err(XeError::new(
                        XeErrorKind::GlobalOutsideFunction,
                        Some(statement.span.clone()),
                    ));
                }
                StatementKind::Global {
                    names: names
                        .iter()
                        .map(|name| {
                            self.module_symbol(name, resolver)
                                .unwrap_or_else(|| name.clone())
                        })
                        .collect(),
                }
            }
            StatementKind::Assignment { name, value } => {
                let value = self.rewrite_expression(value, resolver, scopes, ctx)?;
                StatementKind::Assignment {
                    name: self.resolve_target(name, resolver, scopes, ctx),
                    value,
                }
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => StatementKind::If {
                condition: self.rewrite_expression(condition, resolver, scopes, ctx)?,
                then_block: self.rewrite_statement_block(then_block, resolver, scopes, ctx)?,
                else_block: match else_block {
                    Some(block) => Some(self.rewrite_statement_block(block, resolver, scopes, ctx)?),
                    None => None,
                },
            },
            StatementKind::While { condition, body } => StatementKind::While {
                condition: self.rewrite_expression(condition, resolver, scopes, ctx)?,
                body: self.rewrite_statement_block(body, resolver, scopes, ctx)?,
            },
            StatementKind::Repeat { count, body } => StatementKind::Repeat {
                count: self.rewrite_expression(count, resolver, scopes, ctx)?,
                body: self.rewrite_statement_block(body, resolver, scopes, ctx)?,
            },
            StatementKind::For {
                variable,
                iterable,
                body,
            } => {
                if ctx.is_some_and(|ctx| ctx.globals.contains(variable)) {
                    return Err(XeError::new(
                        XeErrorKind::InvalidGlobal(format!(
                            "'{}' is declared global and cannot be used as a loop variable",
                            variable
                        )),
                        Some(statement.span.clone()),
                    ));
                }
                let iterable = self.rewrite_expression(iterable, resolver, scopes, ctx)?;
                scopes.push(HashSet::from([variable.clone()]));
                let body = self.rewrite_statement_block(body, resolver, scopes, ctx);
                scopes.pop();
                StatementKind::For {
                    variable: variable.clone(),
                    iterable,
                    body: body?,
                }
            }
            StatementKind::Try {
                body,
                catch_variable,
                handler,
            } => StatementKind::Try {
                body: self.rewrite_statement_block(body, resolver, scopes, ctx)?,
                catch_variable: catch_variable
                    .as_ref()
                    .map(|name| self.resolve_target(name, resolver, scopes, ctx)),
                handler: self.rewrite_statement_block(handler, resolver, scopes, ctx)?,
            },
            StatementKind::FunctionDef { .. } | StatementKind::StructDef { .. } => {
                return Err(XeError::new(
                    XeErrorKind::NestedDefinition,
                    Some(statement.span.clone()),
                ));
            }
            StatementKind::Return { value } => StatementKind::Return {
                value: match value {
                    Some(expression) => {
                        Some(self.rewrite_expression(expression, resolver, scopes, ctx)?)
                    }
                    None => None,
                },
            },
            StatementKind::Break => StatementKind::Break,
            StatementKind::Continue => StatementKind::Continue,
            StatementKind::Pass => StatementKind::Pass,
            StatementKind::Expression(expression) => StatementKind::Expression(
                self.rewrite_expression(expression, resolver, scopes, ctx)?,
            ),
            StatementKind::IndexAssignment {
                object,
                index,
                value,
                op,
            } => StatementKind::IndexAssignment {
                object: self.rewrite_expression(object, resolver, scopes, ctx)?,
                index: self.rewrite_expression(index, resolver, scopes, ctx)?,
                value: self.rewrite_expression(value, resolver, scopes, ctx)?,
                op: *op,
            },
            StatementKind::FieldAssignment {
                object,
                field,
                value,
                op,
            } => {
                let value = self.rewrite_expression(value, resolver, scopes, ctx)?;
                // `module.x = v` assigns another module's variable.
                if let Some((module_id, rest)) = self.split_module_access(object, resolver, scopes) {
                    if rest.is_empty() {
                        let symbol = self.module_member_symbol(module_id, field, &statement.span)?;
                        let value = match op {
                            Some(op) => Expression {
                                span: value.span.clone(),
                                kind: ExpressionKind::BinaryOp {
                                    left: Box::new(Expression {
                                        kind: ExpressionKind::Identifier(symbol.clone()),
                                        span: statement.span.clone(),
                                    }),
                                    op: *op,
                                    right: Box::new(value),
                                },
                            },
                            None => value,
                        };
                        return Ok(Statement {
                            kind: StatementKind::Assignment {
                                name: symbol,
                                value,
                            },
                            span: statement.span.clone(),
                        });
                    }
                }
                StatementKind::FieldAssignment {
                    object: self.rewrite_expression(object, resolver, scopes, ctx)?,
                    field: field.clone(),
                    value,
                    op: *op,
                }
            }
        };

        Ok(Statement {
            kind,
            span: statement.span.clone(),
        })
    }

    fn rewrite_expressions(
        &self,
        expressions: &[Expression],
        resolver: &Resolver,
        scopes: &[HashSet<String>],
        ctx: Option<&FunctionContext>,
    ) -> Result<Vec<Expression>, XeError> {
        expressions
            .iter()
            .map(|expression| self.rewrite_expression(expression, resolver, scopes, ctx))
            .collect()
    }

    fn rewrite_expression(
        &self,
        expression: &Expression,
        resolver: &Resolver,
        scopes: &[HashSet<String>],
        ctx: Option<&FunctionContext>,
    ) -> Result<Expression, XeError> {
        let rewrite = |expression: &Expression| self.rewrite_expression(expression, resolver, scopes, ctx);
        let span = &expression.span;
        let kind = match &expression.kind {
            ExpressionKind::Number(value) => ExpressionKind::Number(*value),
            ExpressionKind::String(value) => ExpressionKind::String(value.clone()),
            ExpressionKind::Boolean(value) => ExpressionKind::Boolean(*value),
            ExpressionKind::None => ExpressionKind::None,
            ExpressionKind::List(elements) => {
                ExpressionKind::List(self.rewrite_expressions(elements, resolver, scopes, ctx)?)
            }
            ExpressionKind::Map(entries) => ExpressionKind::Map(
                entries
                    .iter()
                    .map(|(k, v)| Ok((rewrite(k)?, rewrite(v)?)))
                    .collect::<Result<_, XeError>>()?,
            ),
            ExpressionKind::Identifier(name) => {
                ExpressionKind::Identifier(self.resolve_read(name, span, resolver, scopes, ctx)?)
            }
            ExpressionKind::BinaryOp { left, op, right } => ExpressionKind::BinaryOp {
                left: Box::new(rewrite(left)?),
                op: *op,
                right: Box::new(rewrite(right)?),
            },
            ExpressionKind::UnaryOp { op, operand } => ExpressionKind::UnaryOp {
                op: *op,
                operand: Box::new(rewrite(operand)?),
            },
            ExpressionKind::FunctionCall { name, args } => ExpressionKind::FunctionCall {
                name: self.resolve_read(name, span, resolver, scopes, ctx)?,
                args: self.rewrite_expressions(args, resolver, scopes, ctx)?,
            },
            ExpressionKind::MethodCall {
                object,
                method,
                args,
            } => {
                let args = self.rewrite_expressions(args, resolver, scopes, ctx)?;
                match self.split_module_access(object, resolver, scopes) {
                    // `module.function(args)`
                    Some((module_id, rest)) if rest.is_empty() => ExpressionKind::FunctionCall {
                        name: self.module_member_symbol(module_id, method, span)?,
                        args,
                    },
                    _ => ExpressionKind::MethodCall {
                        object: Box::new(rewrite(object)?),
                        method: self.resolve_method(method, resolver),
                        args,
                    },
                }
            }
            ExpressionKind::Call { callee, args } => ExpressionKind::Call {
                callee: Box::new(rewrite(callee)?),
                args: self.rewrite_expressions(args, resolver, scopes, ctx)?,
            },
            ExpressionKind::Lambda { params, body } => {
                let mut inner_scopes = scopes.to_vec();
                inner_scopes.push(params.iter().cloned().collect());
                ExpressionKind::Lambda {
                    params: params.clone(),
                    body: Box::new(self.rewrite_expression(body, resolver, &inner_scopes, ctx)?),
                }
            }
            ExpressionKind::Index { object, index } => ExpressionKind::Index {
                object: Box::new(rewrite(object)?),
                index: Box::new(rewrite(index)?),
            },
            ExpressionKind::Slice { object, start, end } => ExpressionKind::Slice {
                object: Box::new(rewrite(object)?),
                start: match start {
                    Some(start) => Some(Box::new(rewrite(start)?)),
                    None => None,
                },
                end: match end {
                    Some(end) => Some(Box::new(rewrite(end)?)),
                    None => None,
                },
            },
            ExpressionKind::FieldAccess { object, field } => {
                if let Some(rewritten) = self.rewrite_module_access(expression, resolver, scopes)? {
                    return Ok(rewritten);
                }
                ExpressionKind::FieldAccess {
                    object: Box::new(rewrite(object)?),
                    field: field.clone(),
                }
            }
        };

        Ok(Expression {
            kind,
            span: span.clone(),
        })
    }

    fn initialization_order(&self, entry_id: usize) -> Vec<usize> {
        let mut visited = HashSet::new();
        let mut emitted = HashSet::new();
        let mut ordered = Vec::new();
        self.visit_module_dependencies(entry_id, &mut visited, &mut emitted, &mut ordered);
        ordered
            .into_iter()
            .filter(|module_id| *module_id != entry_id)
            .collect()
    }

    fn visit_module_dependencies(
        &self,
        module_id: usize,
        visited: &mut HashSet<usize>,
        emitted: &mut HashSet<usize>,
        ordered: &mut Vec<usize>,
    ) {
        if !visited.insert(module_id) {
            return;
        }

        let module = &self.modules[&module_id];
        for import in &module.imports {
            self.visit_module_dependencies(import.module_id, visited, emitted, ordered);
            if emitted.insert(import.module_id) {
                ordered.push(import.module_id);
            }
        }
    }

    fn failure_for_error(&self, error: XeError) -> CompilationFailure {
        let source = error
            .span
            .as_ref()
            .and_then(|span| span.source_name.as_ref())
            .and_then(|source_name| self.sources.get(source_name))
            .cloned()
            .unwrap_or_default();

        CompilationFailure { error, source }
    }

    fn allocate_module_id(&mut self) -> usize {
        let module_id = self.next_module_id;
        self.next_module_id += 1;
        module_id
    }
}

fn validate_import_placement(program: &Program) -> Result<(), XeError> {
    let mut seen_executable_statement = false;

    for statement in &program.statements {
        match &statement.kind {
            StatementKind::Import { .. } | StatementKind::FromImport { .. } => {
                if seen_executable_statement {
                    return Err(XeError::new(
                        XeErrorKind::ImportAfterExecutableStatement,
                        Some(statement.span.clone()),
                    ));
                }
            }
            StatementKind::FunctionDef { .. } | StatementKind::StructDef { .. } => {}
            _ => seen_executable_statement = true,
        }
    }

    Ok(())
}

/// Imports must be at the top level, and functions and structs may not be nested.
fn validate_nesting(statement: &Statement, is_top_level: bool) -> Result<(), XeError> {
    let nested: Vec<&Vec<Statement>> = match &statement.kind {
        StatementKind::Import { .. } | StatementKind::FromImport { .. } if !is_top_level => {
            return Err(XeError::new(
                XeErrorKind::ImportNotTopLevel,
                Some(statement.span.clone()),
            ));
        }
        StatementKind::FunctionDef { .. } | StatementKind::StructDef { .. } if !is_top_level => {
            return Err(XeError::new(
                XeErrorKind::NestedDefinition,
                Some(statement.span.clone()),
            ));
        }
        StatementKind::If {
            then_block,
            else_block,
            ..
        } => std::iter::once(then_block).chain(else_block.as_ref()).collect(),
        StatementKind::Try { body, handler, .. } => vec![body, handler],
        StatementKind::While { body, .. }
        | StatementKind::Repeat { body, .. }
        | StatementKind::For { body, .. }
        | StatementKind::FunctionDef { body, .. } => vec![body],
        _ => Vec::new(),
    };
    for block in nested {
        for statement in block {
            validate_nesting(statement, false)?;
        }
    }
    Ok(())
}

/// Every module-level name and its linked symbol, plus which of them are variables.
/// Variables are all names assigned in top-level code (including inside blocks) and
/// names declared `global` inside functions.
fn collect_exports(
    module_id: usize,
    program: &Program,
) -> Result<(HashMap<String, String>, HashSet<String>), XeError> {
    let symbol = |name: &str| format!("xe_m{}_{}", module_id, sanitize_symbol(name));
    let mut exports = HashMap::new();
    let mut variables = HashSet::new();

    for statement in &program.statements {
        if let StatementKind::FunctionDef { name, .. } | StatementKind::StructDef { name, .. } =
            &statement.kind
        {
            if exports.insert(name.clone(), symbol(name)).is_some() {
                return Err(XeError::new(
                    XeErrorKind::DuplicateFunction(name.clone()),
                    Some(statement.span.clone()),
                ));
            }
        }
    }

    let mut top_level_targets = Vec::new();
    for statement in &program.statements {
        if !matches!(
            statement.kind,
            StatementKind::FunctionDef { .. } | StatementKind::StructDef { .. }
        ) {
            collect_top_level_targets(statement, &mut top_level_targets);
        }
    }
    let mut declared_globals = Vec::new();
    for statement in &program.statements {
        if let StatementKind::FunctionDef { body, .. } = &statement.kind {
            declared_globals.extend(collect_global_declarations(body));
        }
    }

    for (name, span) in top_level_targets.into_iter().chain(declared_globals) {
        if variables.contains(&name) {
            continue;
        }
        if exports.contains_key(&name) {
            return Err(XeError::new(
                XeErrorKind::InvalidGlobal(format!(
                    "'{}' is a function or struct and cannot also be used as a variable",
                    name
                )),
                Some(span),
            ));
        }
        exports.insert(name.clone(), symbol(&name));
        variables.insert(name);
    }

    Ok((exports, variables))
}

/// Names assigned by top-level code, excluding top-level loop variables (which are
/// scoped to their loop) unless they are also assigned elsewhere.
fn collect_top_level_targets(statement: &Statement, targets: &mut Vec<(String, Span)>) {
    match &statement.kind {
        StatementKind::Assignment { name, .. } => {
            targets.push((name.clone(), statement.span.clone()));
        }
        StatementKind::Try {
            body,
            catch_variable,
            handler,
        } => {
            body.iter().for_each(|s| collect_top_level_targets(s, targets));
            if let Some(name) = catch_variable {
                targets.push((name.clone(), statement.span.clone()));
            }
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
            // Assignments to the loop variable inside its loop do not create a global.
            let mut inner = Vec::new();
            body.iter().for_each(|s| collect_top_level_targets(s, &mut inner));
            targets.extend(inner.into_iter().filter(|(name, _)| name != variable));
        }
        StatementKind::While { body, .. } | StatementKind::Repeat { body, .. } => {
            body.iter().for_each(|s| collect_top_level_targets(s, targets));
        }
        _ => {}
    }
}

/// Names a function assigns (locals) and declares `global`, following Python's rule:
/// a name assigned anywhere in a function is local to it unless declared `global`.
struct FunctionContext {
    assigned: HashSet<String>,
    globals: HashSet<String>,
}

impl FunctionContext {
    fn collect(body: &[Statement]) -> Self {
        let mut assigned = HashSet::new();
        visit_function_body(body, &mut |statement| match &statement.kind {
            StatementKind::Assignment { name, .. } => {
                assigned.insert(name.clone());
            }
            StatementKind::For { variable, .. } => {
                assigned.insert(variable.clone());
            }
            StatementKind::Try {
                catch_variable: Some(name),
                ..
            } => {
                assigned.insert(name.clone());
            }
            _ => {}
        });
        let globals: HashSet<String> = collect_global_declarations(body)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assigned.retain(|name| !globals.contains(name));
        Self { assigned, globals }
    }
}

/// Visits every statement of a function body, descending into nested blocks.
fn visit_function_body(statements: &[Statement], visit: &mut dyn FnMut(&Statement)) {
    for statement in statements {
        visit(statement);
        match &statement.kind {
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                visit_function_body(then_block, visit);
                if let Some(else_block) = else_block {
                    visit_function_body(else_block, visit);
                }
            }
            StatementKind::Try { body, handler, .. } => {
                visit_function_body(body, visit);
                visit_function_body(handler, visit);
            }
            StatementKind::While { body, .. }
            | StatementKind::Repeat { body, .. }
            | StatementKind::For { body, .. } => visit_function_body(body, visit),
            _ => {}
        }
    }
}

fn collect_global_declarations(statements: &[Statement]) -> Vec<(String, Span)> {
    let mut globals = Vec::new();
    visit_function_body(statements, &mut |statement| {
        if let StatementKind::Global { names } = &statement.kind {
            for name in names {
                globals.push((name.clone(), statement.span.clone()));
            }
        }
    });
    globals
}

/// `a.b.c` for an identifier/field chain, used in error messages.
fn expression_path(expression: &Expression) -> String {
    match &expression.kind {
        ExpressionKind::Identifier(name) => name.clone(),
        ExpressionKind::FieldAccess { object, field } => {
            format!("{}.{}", expression_path(object), field)
        }
        _ => "expression".to_string(),
    }
}

fn sanitize_symbol(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    for character in name.chars() {
        if character.is_alphanumeric() || character == '_' {
            output.push(character);
        } else {
            output.push('_');
        }
    }
    output
}

fn is_in_local_scope(scopes: &[HashSet<String>], name: &str) -> bool {
    scopes.iter().skip(1).rev().any(|scope| scope.contains(name))
}
