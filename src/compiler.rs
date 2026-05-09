use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ast::*;
use crate::codegen::CodeGenerator;
use crate::error::{Span, XeError, XeErrorKind};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantic::SemanticAnalyzer;

const BUILTIN_FUNCTIONS: &[&str] = &["print", "input", "length", "type", "convert"];

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
    source_name: String,
    source: String,
    program: Program,
    exports: HashMap<String, String>,
    export_order: Vec<String>,
    imports: Vec<ResolvedImport>,
    init_symbol: String,
}

pub fn compile_path(entry_path: &Path) -> Result<String, CompilationFailure> {
    let mut compiler = ModuleCompiler::new();
    let entry_id = compiler
        .load_entry_module(entry_path)
        .map_err(|error| compiler.failure_for_error(error))?;

    let linked_program = compiler
        .link_program(entry_id)
        .map_err(|error| compiler.failure_for_error(error))?;

    let mut analyzer = SemanticAnalyzer::new();
    analyzer
        .analyze(&linked_program)
        .map_err(|error| compiler.failure_for_error(error))?;

    let mut codegen = CodeGenerator::new();
    Ok(codegen.generate(&linked_program))
}

struct ModuleCompiler {
    next_module_id: usize,
    modules: HashMap<usize, ModuleRecord>,
    module_ids_by_path: HashMap<PathBuf, usize>,
    loading_stack: Vec<PathBuf>,
    sources: HashMap<String, String>,
}

impl ModuleCompiler {
    fn new() -> Self {
        Self {
            next_module_id: 0,
            modules: HashMap::new(),
            module_ids_by_path: HashMap::new(),
            loading_stack: Vec::new(),
            sources: HashMap::new(),
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

        self.loading_stack.push(canonical.clone());

        let result = (|| -> Result<ModuleRecord, XeError> {
            let program = self.parse_program(&source, &source_name)?;
            self.validate_import_placement(&program)?;
            self.validate_no_nested_imports(&program)?;

            let module_id = self.allocate_module_id();
            let exports = self.collect_exports(module_id, &program)?;
            let export_order = self.collect_export_order(&program);
            let imports = self.resolve_imports(&canonical, &program)?;

            Ok(ModuleRecord {
                id: module_id,
                path: canonical.clone(),
                source_name: source_name.clone(),
                source: source.clone(),
                program,
                exports,
                export_order,
                imports,
                init_symbol: format!("xe_m{}_init", module_id),
            })
        })();

        self.loading_stack.pop();

        let module = result?;
        self.sources
            .insert(module.source_name.clone(), module.source.clone());
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
            StatementKind::Import { .. } | StatementKind::FromImport { .. } => {
                if !is_top_level {
                    return Err(XeError::new(
                        XeErrorKind::ImportNotTopLevel,
                        Some(statement.span.clone()),
                    ));
                }
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
    ) -> Result<HashMap<String, String>, XeError> {
        let mut exports = HashMap::new();

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
                StatementKind::Assignment { name, .. } => {
                    // Top-level assignments are also exports
                    if !exports.contains_key(name) && !BUILTIN_FUNCTIONS.contains(&name.as_str()) {
                        exports.insert(
                            name.clone(),
                            format!("xe_m{}_{}", module_id, sanitize_symbol(name)),
                        );
                    }
                }
                _ => {}
            }
        }

        Ok(exports)
    }

    fn collect_export_order(&self, program: &Program) -> Vec<String> {
        let mut export_order = Vec::new();

        for statement in &program.statements {
            if let StatementKind::FunctionDef { name, .. } = &statement.kind {
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

        for module_id in &module_ids {
            let module = self.modules.get(module_id).unwrap();
            let imported_functions = self.build_imported_function_map(module)?;

            for statement in &module.program.statements {
                if let StatementKind::FunctionDef { name, params, body } = &statement.kind {
                    statements.push(Statement {
                        kind: StatementKind::FunctionDef {
                            name: module.exports.get(name).unwrap().clone(),
                            params: params.clone(),
                            body: self.rewrite_statement_block(body, module, &imported_functions),
                        },
                        span: statement.span.clone(),
                    });
                }
            }
        }

        for module_id in self.initialization_order(entry_id) {
            let module = self.modules.get(&module_id).unwrap();
            if !self.module_has_top_level_code(module) {
                continue;
            }

            let imported_functions = self.build_imported_function_map(module)?;
            let body = self.link_top_level_executable_statements(module, &imported_functions);

            statements.push(Statement {
                kind: StatementKind::FunctionDef {
                    name: module.init_symbol.clone(),
                    params: Vec::new(),
                    body,
                },
                span: module
                    .program
                    .statements
                    .first()
                    .map(|statement| statement.span.clone())
                    .unwrap_or_else(|| Span::with_source(1, 1, module.source_name.clone())),
            });
        }

        let entry_module = self.modules.get(&entry_id).unwrap();
        for module_id in self.initialization_order(entry_id) {
            let module = self.modules.get(&module_id).unwrap();
            if self.module_has_top_level_code(module) {
                statements.push(self.make_init_call(entry_module, &module.init_symbol));
            }
        }

        let imported_functions = self.build_imported_function_map(entry_module)?;
        statements
            .extend(self.link_top_level_executable_statements(entry_module, &imported_functions));

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
    ) -> Vec<Statement> {
        statements
            .iter()
            .map(|statement| self.rewrite_statement(statement, module, imported_functions))
            .collect()
    }

    fn rewrite_statement(
        &self,
        statement: &Statement,
        module: &ModuleRecord,
        imported_functions: &HashMap<String, String>,
    ) -> Statement {
        let kind = match &statement.kind {
            StatementKind::Import { .. } | StatementKind::FromImport { .. } => {
                statement.kind.clone()
            }
            StatementKind::Assignment { name, value } => {
                let rewritten_name = if let Some(exported) = module.exports.get(name) {
                    exported.clone()
                } else if let Some(imported) = imported_functions.get(name) {
                    imported.clone()
                } else {
                    name.clone()
                };
                StatementKind::Assignment {
                    name: rewritten_name,
                    value: self.rewrite_expression(value, module, imported_functions),
                }
            }
            StatementKind::If {
                condition,
                then_block,
                else_block,
            } => StatementKind::If {
                condition: self.rewrite_expression(condition, module, imported_functions),
                then_block: self.rewrite_statement_block(then_block, module, imported_functions),
                else_block: else_block
                    .as_ref()
                    .map(|block| self.rewrite_statement_block(block, module, imported_functions)),
            },
            StatementKind::While { condition, body } => StatementKind::While {
                condition: self.rewrite_expression(condition, module, imported_functions),
                body: self.rewrite_statement_block(body, module, imported_functions),
            },
            StatementKind::Repeat { count, body } => StatementKind::Repeat {
                count: self.rewrite_expression(count, module, imported_functions),
                body: self.rewrite_statement_block(body, module, imported_functions),
            },
            StatementKind::For {
                variable,
                iterable,
                body,
            } => StatementKind::For {
                variable: variable.clone(),
                iterable: self.rewrite_expression(iterable, module, imported_functions),
                body: self.rewrite_statement_block(body, module, imported_functions),
            },
            StatementKind::FunctionDef { name, params, body } => StatementKind::FunctionDef {
                name: module
                    .exports
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| name.clone()),
                params: params.clone(),
                body: self.rewrite_statement_block(body, module, imported_functions),
            },
            StatementKind::Return { value } => StatementKind::Return {
                value: value.as_ref().map(|expression| {
                    self.rewrite_expression(expression, module, imported_functions)
                }),
            },
            StatementKind::Break => StatementKind::Break,
            StatementKind::Continue => StatementKind::Continue,
            StatementKind::Expression(expression) => StatementKind::Expression(
                self.rewrite_expression(expression, module, imported_functions),
            ),
        };

        Statement {
            kind,
            span: statement.span.clone(),
        }
    }

    fn rewrite_expression(
        &self,
        expression: &Expression,
        module: &ModuleRecord,
        imported_functions: &HashMap<String, String>,
    ) -> Expression {
        let kind = match &expression.kind {
            ExpressionKind::Number(value) => ExpressionKind::Number(*value),
            ExpressionKind::String(value) => ExpressionKind::String(value.clone()),
            ExpressionKind::Boolean(value) => ExpressionKind::Boolean(*value),
            ExpressionKind::List(elements) => ExpressionKind::List(
                elements
                    .iter()
                    .map(|element| self.rewrite_expression(element, module, imported_functions))
                    .collect(),
            ),
            ExpressionKind::Identifier(name) => {
                let rewritten_name = if let Some(exported) = module.exports.get(name) {
                    exported.clone()
                } else if let Some(imported) = imported_functions.get(name) {
                    imported.clone()
                } else {
                    name.clone()
                };
                ExpressionKind::Identifier(rewritten_name)
            }
            ExpressionKind::BinaryOp { left, op, right } => ExpressionKind::BinaryOp {
                left: Box::new(self.rewrite_expression(left, module, imported_functions)),
                op: *op,
                right: Box::new(self.rewrite_expression(right, module, imported_functions)),
            },
            ExpressionKind::UnaryOp { op, operand } => ExpressionKind::UnaryOp {
                op: *op,
                operand: Box::new(self.rewrite_expression(operand, module, imported_functions)),
            },
            ExpressionKind::FunctionCall { name, args } => {
                let rewritten_name = if BUILTIN_FUNCTIONS.contains(&name.as_str()) {
                    name.clone()
                } else if let Some(local_symbol) = module.exports.get(name) {
                    local_symbol.clone()
                } else if let Some(imported_symbol) = imported_functions.get(name) {
                    imported_symbol.clone()
                } else {
                    name.clone()
                };

                ExpressionKind::FunctionCall {
                    name: rewritten_name,
                    args: args
                        .iter()
                        .map(|argument| {
                            self.rewrite_expression(argument, module, imported_functions)
                        })
                        .collect(),
                }
            }
            ExpressionKind::Index { object, index } => ExpressionKind::Index {
                object: Box::new(self.rewrite_expression(object, module, imported_functions)),
                index: Box::new(self.rewrite_expression(index, module, imported_functions)),
            },
        };

        Expression {
            kind,
            span: expression.span.clone(),
        }
    }

    fn link_top_level_executable_statements(
        &self,
        module: &ModuleRecord,
        imported_functions: &HashMap<String, String>,
    ) -> Vec<Statement> {
        let mut linked = Vec::new();

        for statement in &module.program.statements {
            match &statement.kind {
                StatementKind::Import { .. }
                | StatementKind::FromImport { .. }
                | StatementKind::FunctionDef { .. } => {}
                _ => linked.push(self.rewrite_statement(statement, module, imported_functions)),
            }
        }

        linked
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

    fn module_has_top_level_code(&self, module: &ModuleRecord) -> bool {
        module.program.statements.iter().any(|statement| {
            !matches!(
                statement.kind,
                StatementKind::Import { .. }
                    | StatementKind::FromImport { .. }
                    | StatementKind::FunctionDef { .. }
            )
        })
    }

    fn make_init_call(&self, entry_module: &ModuleRecord, init_symbol: &str) -> Statement {
        let span = entry_module
            .program
            .statements
            .first()
            .map(|statement| statement.span.clone())
            .unwrap_or_else(|| Span::with_source(1, 1, entry_module.source_name.clone()));

        Statement {
            kind: StatementKind::Expression(Expression {
                kind: ExpressionKind::FunctionCall {
                    name: init_symbol.to_string(),
                    args: Vec::new(),
                },
                span: span.clone(),
            }),
            span,
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

fn sanitize_symbol(name: &str) -> String {
    let mut output = String::with_capacity(name.len());
    for character in name.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            output.push(character);
        } else {
            output.push('_');
        }
    }
    output
}
