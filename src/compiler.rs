use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::*;
use crate::codegen::CodeGenerator;
use crate::error::{Span, XeError, XeErrorKind};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantic::SemanticAnalyzer;

const BUILTIN_FUNCTIONS: &[&str] = &[
    "print", "input", "length", "type", "convert",
    "append", "pop", "keys", "values", "has_key", "contains", "split", "join"
];

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
    All,
    Names(Vec<String>),
}

struct ModuleRecord {
    id: usize,
    path: PathBuf,
    program: Program,
    exports: HashMap<String, String>,
    export_order: Vec<String>,
    imports: Vec<ResolvedImport>,
}

pub fn compile_path(entry_path: &Path) -> Result<String, Box<CompilationFailure>> {
    let mut compiler = ModuleCompiler::new();
    let entry_id = compiler
        .load_entry_module(entry_path)
        .map_err(|error| Box::new(compiler.failure_for_error(error)))?;

    let linked_program = compiler
        .link_program(entry_id)
        .map_err(|error| Box::new(compiler.failure_for_error(error)))?;

    let mut analyzer = SemanticAnalyzer::new();
    let typed_program = analyzer
        .analyze(&linked_program)
        .map_err(|error| Box::new(compiler.failure_for_error(error)))?;

    let mut codegen = CodeGenerator::new();
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
            self.validate_import_placement(&program)?;
            self.validate_no_nested_imports(&program)?;

            let module_id = self.allocate_module_id();
            let (exports, variables) = self.collect_exports(module_id, &program)?;
            for variable in &variables {
                self.variable_symbols.insert(exports[variable].clone());
            }
            let export_order = self.collect_export_order(&program);
            let imports = self.resolve_imports(&canonical, &program)?;

            Ok(ModuleRecord {
                id: module_id,
                path: path.to_path_buf(),
                program,
                exports,
                export_order,
                imports,
            })
        })();

        self.loading_stack.pop();

        let module = result?;
        self.module_ids_by_path
            .insert(module.path.clone(), module.id);
        self.modules.insert(module.id, module);

        Ok(*self.module_ids_by_path.get(&canonical).unwrap())
    }

    fn parse_program(&self, source: &str, source_name: &str) -> Result<Program, XeError> {
        let mut lexer = Lexer::new_with_source(source, Some(source_name.to_string()));
        let tokens = lexer.tokenize()?;

        let mut parser = Parser::new(tokens);
        parser.parse()
    }

    fn validate_import_placement(&self, program: &Program) -> Result<(), XeError> {
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
                StatementKind::FunctionDef { .. } => {}
                _ => {
                    seen_executable_statement = true;
                }
            }
        }

        Ok(())
    }

    fn validate_no_nested_imports(&self, program: &Program) -> Result<(), XeError> {
        for statement in &program.statements {
            self.validate_statement_is_not_nested_import(statement, true)?;
        }

        Ok(())
    }

    fn validate_statement_is_not_nested_import(
        &self,
        statement: &Statement,
        is_top_level: bool,
    ) -> Result<(), XeError> {
        match &statement.kind {
            StatementKind::Import { .. } | StatementKind::FromImport { .. } if !is_top_level => {
                return Err(XeError::new(
                    XeErrorKind::ImportNotTopLevel,
                    Some(statement.span.clone()),
                ));
            }
            StatementKind::If {
                then_block,
                else_block,
                ..
            } => {
                for nested in then_block {
                    self.validate_statement_is_not_nested_import(nested, false)?;
                }
                if let Some(else_block) = else_block {
                    for nested in else_block {
                        self.validate_statement_is_not_nested_import(nested, false)?;
                    }
                }
            }
            StatementKind::While { body, .. }
            | StatementKind::Repeat { body, .. }
            | StatementKind::For { body, .. }
            | StatementKind::FunctionDef { body, .. } => {
                for nested in body {
                    self.validate_statement_is_not_nested_import(nested, false)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn collect_exports(
        &self,
        module_id: usize,
        program: &Program,
    ) -> Result<(HashMap<String, String>, HashSet<String>), XeError> {
        let mut exports = HashMap::new();
        let mut variables = HashSet::new();

        for statement in &program.statements {
            match &statement.kind {
                StatementKind::FunctionDef { name, .. } => {
                    if BUILTIN_FUNCTIONS.contains(&name.as_str()) {
                        return Err(XeError::new(
                            XeErrorKind::CannotRedefineBuiltin(name.clone()),
                            Some(statement.span.clone()),
                        ));
                    }

                    if exports.contains_key(name) {
                        return Err(XeError::new(
                            XeErrorKind::DuplicateFunction(name.clone()),
                            Some(statement.span.clone()),
                        ));
                    }

                    exports.insert(
                        name.clone(),
                        format!("xe_m{}_{}", module_id, sanitize_symbol(name)),
                    );
                }
                StatementKind::StructDef { name, .. } => {
                    if BUILTIN_FUNCTIONS.contains(&name.as_str()) {
                        return Err(XeError::new(
                            XeErrorKind::CannotRedefineBuiltin(name.clone()),
                            Some(statement.span.clone()),
                        ));
                    }

                    if exports.contains_key(name) {
                        return Err(XeError::new(
                            XeErrorKind::DuplicateFunction(name.clone()),
                            Some(statement.span.clone()),
                        ));
                    }

                    exports.insert(
                        name.clone(),
                        format!("xe_m{}_{}", module_id, sanitize_symbol(name)),
                    );
                }
                StatementKind::Assignment { name, .. }
                    if !exports.contains_key(name)
                        && !BUILTIN_FUNCTIONS.contains(&name.as_str()) =>
                {
                    exports.insert(
                        name.clone(),
                        format!("xe_m{}_{}", module_id, sanitize_symbol(name)),
                    );
                    variables.insert(name.clone());
                }
                _ => {}
            }
        }

        // Names declared `global` inside functions are module-level variables too.
        for statement in &program.statements {
            if let StatementKind::FunctionDef { body, .. } = &statement.kind {
                for (name, span) in collect_global_declarations(body) {
                    if variables.contains(&name) {
                        continue;
                    }
                    if exports.contains_key(&name) || BUILTIN_FUNCTIONS.contains(&name.as_str()) {
                        return Err(XeError::new(
                            XeErrorKind::InvalidGlobal(format!(
                                "'{}' is a function or struct and cannot be declared as a global variable",
                                name
                            )),
                            Some(span),
                        ));
                    }
                    exports.insert(
                        name.clone(),
                        format!("xe_m{}_{}", module_id, sanitize_symbol(&name)),
                    );
                    variables.insert(name);
                }
            }
        }

        Ok((exports, variables))
    }

    fn collect_export_order(&self, program: &Program) -> Vec<String> {
        let mut export_order = Vec::new();

        for statement in &program.statements {
            if let StatementKind::FunctionDef { name, .. } = &statement.kind {
                export_order.push(name.clone());
            } else if let StatementKind::StructDef { name, .. } = &statement.kind {
                export_order.push(name.clone());
            }
        }

        export_order
    }

    fn resolve_imports(
        &mut self,
        module_path: &Path,
        program: &Program,
    ) -> Result<Vec<ResolvedImport>, XeError> {
        let module_dir = module_path.parent().unwrap_or_else(|| Path::new("."));
        let mut imports = Vec::new();

        for statement in &program.statements {
            match &statement.kind {
                StatementKind::Import { module } => {
                    let dependency_path =
                        self.resolve_module_path(module_dir, module, &statement.span)?;
                    let dependency_id =
                        self.load_module(&dependency_path, Some(&statement.span))?;
                    imports.push(ResolvedImport {
                        module_id: dependency_id,
                        kind: ResolvedImportKind::All,
                        span: statement.span.clone(),
                    });
                }
                StatementKind::FromImport { module, names } => {
                    let dependency_path =
                        self.resolve_module_path(module_dir, module, &statement.span)?;
                    let dependency_id =
                        self.load_module(&dependency_path, Some(&statement.span))?;
                    imports.push(ResolvedImport {
                        module_id: dependency_id,
                        kind: ResolvedImportKind::Names(names.clone()),
                        span: statement.span.clone(),
                    });
                }
                _ => {}
            }
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

        // 1. Collect all function definitions from all modules
        for module_id in &module_ids {
            let module = self.modules.get(module_id).unwrap();
            let imported_functions = self.build_imported_function_map(module)?;

            for statement in &module.program.statements {
                if let StatementKind::FunctionDef { .. } = &statement.kind {
                    statements.push(Statement {
                        kind: self.rewrite_function_def(statement, module, &imported_functions)?,
                        span: statement.span.clone(),
                    });
                } else if let StatementKind::StructDef { name, fields } = &statement.kind {
                    statements.push(Statement {
                        kind: StatementKind::StructDef {
                            name: module.exports.get(name).unwrap().clone(),
                            fields: fields.clone(),
                        },
                        span: statement.span.clone(),
                    });
                }
            }
        }

        // 2. Collect and flatten all top-level executable statements in dependency order
        for module_id in self.initialization_order(entry_id) {
            let module = self.modules.get(&module_id).unwrap();
            let imported_functions = self.build_imported_function_map(module)?;
            let body = self.link_top_level_executable_statements(module, &imported_functions)?;
            statements.extend(body);
        }

        // 3. Finally, add the entry module's own top-level code
        let entry_module = self.modules.get(&entry_id).unwrap();
        let imported_functions = self.build_imported_function_map(entry_module)?;
        statements
            .extend(self.link_top_level_executable_statements(entry_module, &imported_functions)?);

        Ok(Program { statements })
    }

    fn build_imported_function_map(
        &self,
        module: &ModuleRecord,
    ) -> Result<HashMap<String, String>, XeError> {
        let mut imported_functions = HashMap::new();

        for import in &module.imports {
            let dependency = self.modules.get(&import.module_id).unwrap();
            match &import.kind {
                ResolvedImportKind::All => {
                    for export_name in &dependency.export_order {
                        self.insert_imported_name(
                            &mut imported_functions,
                            module,
                            dependency,
                            export_name,
                            export_name,
                            &import.span,
                        )?;
                    }
                }
                ResolvedImportKind::Names(names) => {
                    for name in names {
                        self.insert_imported_name(
                            &mut imported_functions,
                            module,
                            dependency,
                            name,
                            name,
                            &import.span,
                        )?;
                    }
                }
            }
        }

        Ok(imported_functions)
    }

    fn insert_imported_name(
        &self,
        imported_functions: &mut HashMap<String, String>,
        module: &ModuleRecord,
        dependency: &ModuleRecord,
        imported_name: &str,
        local_name: &str,
        span: &Span,
    ) -> Result<(), XeError> {
        let Some(target_symbol) = dependency.exports.get(imported_name) else {
            return Err(XeError::new(
                XeErrorKind::ImportedNameNotFound {
                    module: dependency.path.display().to_string(),
                    name: imported_name.to_string(),
                },
                Some(span.clone()),
            ));
        };

        if module.exports.contains_key(local_name) || BUILTIN_FUNCTIONS.contains(&local_name) {
            return Err(XeError::new(
                XeErrorKind::ImportNameConflict(local_name.to_string()),
                Some(span.clone()),
            ));
        }

        if imported_functions.contains_key(local_name) {
            return Err(XeError::new(
                XeErrorKind::DuplicateImport(local_name.to_string()),
                Some(span.clone()),
            ));
        }

        imported_functions.insert(local_name.to_string(), target_symbol.clone());
        Ok(())
    }

    fn rewrite_statement_block(
        &self,
        statements: &[Statement],
        module: &ModuleRecord,
        imported_functions: &HashMap<String, String>,
        scopes: &mut Vec<HashSet<String>>,
        ctx: Option<&FunctionContext>,
    ) -> Result<Vec<Statement>, XeError> {
        statements
            .iter()
            .map(|statement| {
                self.rewrite_statement(statement, module, imported_functions, scopes, ctx)
            })
            .collect()
    }

    /// Resolves a module-level name (own export or import) to its linked symbol.
    fn module_symbol(
        &self,
        name: &str,
        module: &ModuleRecord,
        imported_functions: &HashMap<String, String>,
    ) -> Option<String> {
        module
            .exports
            .get(name)
            .or_else(|| imported_functions.get(name))
            .cloned()
    }

    fn rewrite_function_def(
        &self,
        statement: &Statement,
        module: &ModuleRecord,
        imported_functions: &HashMap<String, String>,
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
        let rewritten_body = self.rewrite_statement_block(
            body,
            module,
            imported_functions,
            &mut scopes,
            Some(&ctx),
        )?;

        Ok(StatementKind::FunctionDef {
            name: module
                .exports
                .get(name)
                .cloned()
                .unwrap_or_else(|| name.clone()),
            params: params.clone(),
            body: rewritten_body,
        })
    }

    fn rewrite_statement(
        &self,
        statement: &Statement,
        module: &ModuleRecord,
        imported_functions: &HashMap<String, String>,
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
                            self.module_symbol(name, module, imported_functions)
                                .unwrap_or_else(|| name.clone())
                        })
                        .collect(),
                }
            }
            StatementKind::Assignment { name, value } => {
                let rewritten_value =
                    self.rewrite_expression(value, module, imported_functions, scopes, ctx)?;
                let rewritten_name = match ctx {
                    Some(ctx) if ctx.globals.contains(name) => self
                        .module_symbol(name, module, imported_functions)
                        .unwrap_or_else(|| name.clone()),
                    Some(_) => {
                        if !is_in_local_scope(scopes, name) {
                            scopes.last_mut().unwrap().insert(name.clone());
                        }
                        name.clone()
                    }
                    None => {
                        if is_in_local_scope(scopes, name) {
                            name.clone()
                        } else {
                            self.module_symbol(name, module, imported_functions)
                                .unwrap_or_else(|| name.clone())
                        }
                    }
                };
                StatementKind::Assignment {
                    name: rewritten_name,
                    value: rewritten_value,
                }
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => StatementKind::If {
                condition: self.rewrite_expression(
                    condition,
                    module,
                    imported_functions,
                    scopes,
                    ctx,
                )?,
                then_block: self.rewrite_statement_block(
                    then_block,
                    module,
                    imported_functions,
                    scopes,
                    ctx,
                )?,
                else_block: match else_block {
                    Some(block) => Some(self.rewrite_statement_block(
                        block,
                        module,
                        imported_functions,
                        scopes,
                        ctx,
                    )?),
                    None => None,
                },
            },
            StatementKind::While { condition, body } => StatementKind::While {
                condition: self.rewrite_expression(
                    condition,
                    module,
                    imported_functions,
                    scopes,
                    ctx,
                )?,
                body: self.rewrite_statement_block(body, module, imported_functions, scopes, ctx)?,
            },
            StatementKind::Repeat { count, body } => StatementKind::Repeat {
                count: self.rewrite_expression(count, module, imported_functions, scopes, ctx)?,
                body: self.rewrite_statement_block(body, module, imported_functions, scopes, ctx)?,
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
                let rewritten_iterable =
                    self.rewrite_expression(iterable, module, imported_functions, scopes, ctx)?;
                scopes.push(HashSet::new());
                scopes.last_mut().unwrap().insert(variable.clone());
                let rewritten_body =
                    self.rewrite_statement_block(body, module, imported_functions, scopes, ctx);
                scopes.pop();
                StatementKind::For {
                    variable: variable.clone(),
                    iterable: rewritten_iterable,
                    body: rewritten_body?,
                }
            }
            StatementKind::FunctionDef { .. } => {
                self.rewrite_function_def(statement, module, imported_functions)?
            }
            StatementKind::Return { value } => StatementKind::Return {
                value: match value {
                    Some(expression) => Some(self.rewrite_expression(
                        expression,
                        module,
                        imported_functions,
                        scopes,
                        ctx,
                    )?),
                    None => None,
                },
            },
            StatementKind::Break => StatementKind::Break,
            StatementKind::Continue => StatementKind::Continue,
            StatementKind::Expression(expression) => StatementKind::Expression(
                self.rewrite_expression(expression, module, imported_functions, scopes, ctx)?,
            ),
            StatementKind::StructDef { name, fields } => {
                let rewritten_name = module
                    .exports
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| name.clone());
                StatementKind::StructDef {
                    name: rewritten_name,
                    fields: fields.clone(),
                }
            }
            StatementKind::IndexAssignment { object, index, value } => {
                StatementKind::IndexAssignment {
                    object: self.rewrite_expression(object, module, imported_functions, scopes, ctx)?,
                    index: self.rewrite_expression(index, module, imported_functions, scopes, ctx)?,
                    value: self.rewrite_expression(value, module, imported_functions, scopes, ctx)?,
                }
            }
            StatementKind::FieldAssignment { object, field, value } => {
                StatementKind::FieldAssignment {
                    object: self.rewrite_expression(object, module, imported_functions, scopes, ctx)?,
                    field: field.clone(),
                    value: self.rewrite_expression(value, module, imported_functions, scopes, ctx)?,
                }
            }
        };

        Ok(Statement {
            kind,
            span: statement.span.clone(),
        })
    }

    fn rewrite_expression(
        &self,
        expression: &Expression,
        module: &ModuleRecord,
        imported_functions: &HashMap<String, String>,
        scopes: &[HashSet<String>],
        ctx: Option<&FunctionContext>,
    ) -> Result<Expression, XeError> {
        let rewrite = |expression: &Expression| {
            self.rewrite_expression(expression, module, imported_functions, scopes, ctx)
        };
        let kind = match &expression.kind {
            ExpressionKind::Number(value) => ExpressionKind::Number(*value),
            ExpressionKind::String(value) => ExpressionKind::String(value.clone()),
            ExpressionKind::Boolean(value) => ExpressionKind::Boolean(*value),
            ExpressionKind::List(elements) => {
                ExpressionKind::List(elements.iter().map(rewrite).collect::<Result<_, _>>()?)
            }
            ExpressionKind::Identifier(name) => {
                let rewritten_name = if is_in_local_scope(scopes, name) {
                    name.clone()
                } else if ctx.is_some_and(|ctx| ctx.globals.contains(name)) {
                    self.module_symbol(name, module, imported_functions)
                        .unwrap_or_else(|| name.clone())
                } else if ctx.is_some_and(|ctx| ctx.assigned.contains(name)) {
                    // Assigned somewhere in this function, so the name is local everywhere in it.
                    let is_module_variable = self
                        .module_symbol(name, module, imported_functions)
                        .is_some_and(|symbol| self.variable_symbols.contains(&symbol));
                    if is_module_variable {
                        return Err(XeError::new(
                            XeErrorKind::LocalUsedBeforeAssignment(name.clone()),
                            Some(expression.span.clone()),
                        ));
                    }
                    name.clone()
                } else {
                    self.module_symbol(name, module, imported_functions)
                        .unwrap_or_else(|| name.clone())
                };
                ExpressionKind::Identifier(rewritten_name)
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
            ExpressionKind::FunctionCall { name, args } => {
                let rewritten_name = if BUILTIN_FUNCTIONS.contains(&name.as_str())
                    || is_in_local_scope(scopes, name)
                {
                    name.clone()
                } else {
                    self.module_symbol(name, module, imported_functions)
                        .unwrap_or_else(|| name.clone())
                };

                ExpressionKind::FunctionCall {
                    name: rewritten_name,
                    args: args.iter().map(rewrite).collect::<Result<_, _>>()?,
                }
            }
            ExpressionKind::Index { object, index } => ExpressionKind::Index {
                object: Box::new(rewrite(object)?),
                index: Box::new(rewrite(index)?),
            },
            ExpressionKind::Map(entries) => ExpressionKind::Map(
                entries
                    .iter()
                    .map(|(k, v)| Ok((rewrite(k)?, rewrite(v)?)))
                    .collect::<Result<_, XeError>>()?,
            ),
            ExpressionKind::FieldAccess { object, field } => ExpressionKind::FieldAccess {
                object: Box::new(rewrite(object)?),
                field: field.clone(),
            },
        };

        Ok(Expression {
            kind,
            span: expression.span.clone(),
        })
    }

    fn link_top_level_executable_statements(
        &self,
        module: &ModuleRecord,
        imported_functions: &HashMap<String, String>,
    ) -> Result<Vec<Statement>, XeError> {
        let mut linked = Vec::new();
        let mut scopes = vec![HashSet::new()];

        for statement in &module.program.statements {
            match &statement.kind {
                StatementKind::Import { .. }
                | StatementKind::FromImport { .. }
                | StatementKind::FunctionDef { .. }
                | StatementKind::StructDef { .. } => {}
                _ => linked.push(self.rewrite_statement(
                    statement,
                    module,
                    imported_functions,
                    &mut scopes,
                    None,
                )?),
            }
        }

        Ok(linked)
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

        let module = self.modules.get(&module_id).unwrap();
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

/// Names a function assigns (locals) and declares `global`, following Python's rule:
/// a name assigned anywhere in a function is local to it unless declared `global`.
struct FunctionContext {
    assigned: HashSet<String>,
    globals: HashSet<String>,
}

impl FunctionContext {
    fn collect(body: &[Statement]) -> Self {
        let mut assigned = HashSet::new();
        collect_assigned_names(body, &mut assigned);
        let globals: HashSet<String> = collect_global_declarations(body)
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assigned.retain(|name| !globals.contains(name));
        Self { assigned, globals }
    }
}

/// Visits every statement of a function body, descending into nested blocks
/// but not into nested function definitions.
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
            StatementKind::While { body, .. }
            | StatementKind::Repeat { body, .. }
            | StatementKind::For { body, .. } => visit_function_body(body, visit),
            _ => {}
        }
    }
}

fn collect_assigned_names(statements: &[Statement], assigned: &mut HashSet<String>) {
    visit_function_body(statements, &mut |statement| match &statement.kind {
        StatementKind::Assignment { name, .. } => {
            assigned.insert(name.clone());
        }
        StatementKind::For { variable, .. } => {
            assigned.insert(variable.clone());
        }
        _ => {}
    });
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
    for scope in scopes.iter().skip(1).rev() {
        if scope.contains(name) {
            return true;
        }
    }
    false
}

