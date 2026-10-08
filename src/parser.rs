use crate::ast::*;
use crate::error::{Span, XeError, XeErrorKind, XeResult};
use crate::lexer::{Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(&mut self) -> XeResult<Program> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            self.skip_newlines();
            if self.is_at_end() {
                break;
            }
            statements.push(self.parse_statement()?);
        }

        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> XeResult<Statement> {
        let span = self.current_span();

        match self.peek_kind() {
            TokenKind::Import => return self.parse_import_statement(),
            TokenKind::From => return self.parse_from_import_statement(),
            TokenKind::Function => return self.parse_function_def(),
            TokenKind::If => return self.parse_if_like_statement(TokenKind::If),
            TokenKind::While => return self.parse_while_statement(),
            TokenKind::Repeat => return self.parse_repeat_statement(),
            TokenKind::For => return self.parse_for_statement(),
            TokenKind::Return => return self.parse_return_statement(),
            TokenKind::Struct => return self.parse_struct_def(),
            TokenKind::Try => return self.parse_try_statement(),
            TokenKind::Break | TokenKind::Continue | TokenKind::Pass => {
                let kind = match self.peek_kind() {
                    TokenKind::Break => StatementKind::Break,
                    TokenKind::Continue => StatementKind::Continue,
                    _ => StatementKind::Pass,
                };
                self.advance();
                self.expect_statement_end()?;
                return Ok(Statement { kind, span });
            }
            TokenKind::Global => {
                self.advance();
                let mut names = vec![self.expect_identifier()?];
                while self.match_token(&TokenKind::Comma) {
                    names.push(self.expect_identifier()?);
                }
                self.expect_statement_end()?;
                return Ok(Statement {
                    kind: StatementKind::Global { names },
                    span,
                });
            }
            _ => {}
        }

        // Assignment, augmented assignment, or expression statement
        let expr = self.parse_expression()?;

        let op = match self.peek_kind() {
            TokenKind::Equal => None,
            kind => match augmented_operator(kind) {
                Some(op) => Some(op),
                None => {
                    self.expect_statement_end()?;
                    return Ok(Statement {
                        kind: StatementKind::Expression(expr),
                        span,
                    });
                }
            },
        };
        self.advance(); // consume '=' or 'op='
        let value = self.parse_expression()?;
        self.expect_statement_end()?;

        let kind = match expr.kind {
            ExpressionKind::Identifier(name) => {
                let value = match op {
                    // `x op= v` is `x = x op v`; reading a variable has no side effects.
                    Some(op) => Expression {
                        span: value.span.clone(),
                        kind: ExpressionKind::BinaryOp {
                            left: Box::new(Expression {
                                kind: ExpressionKind::Identifier(name.clone()),
                                span: expr.span.clone(),
                            }),
                            op,
                            right: Box::new(value),
                        },
                    },
                    None => value,
                };
                StatementKind::Assignment { name, value }
            }
            ExpressionKind::Index { object, index } => StatementKind::IndexAssignment {
                object: *object,
                index: *index,
                value,
                op,
            },
            ExpressionKind::FieldAccess { object, field } => StatementKind::FieldAssignment {
                object: *object,
                field,
                value,
                op,
            },
            _ => {
                return Err(XeError::new(
                    XeErrorKind::ExpectedToken("valid assignment target".to_string()),
                    Some(span),
                ));
            }
        };

        Ok(Statement { kind, span })
    }

    fn parse_import_statement(&mut self) -> XeResult<Statement> {
        let span = self.current_span();
        self.advance(); // consume 'import'
        let module = self.parse_module_path()?;
        let alias = if self.match_token(&TokenKind::As) {
            Some(self.expect_identifier()?)
        } else {
            None
        };
        self.expect_statement_end()?;
        Ok(Statement {
            kind: StatementKind::Import { module, alias },
            span,
        })
    }

    fn parse_from_import_statement(&mut self) -> XeResult<Statement> {
        let span = self.current_span();
        self.advance(); // consume 'from'
        let module = self.parse_module_path()?;
        self.expect(&TokenKind::Import)?;

        let mut names = vec![self.parse_import_name()?];
        while self.match_token(&TokenKind::Comma) {
            names.push(self.parse_import_name()?);
        }

        self.expect_statement_end()?;
        Ok(Statement {
            kind: StatementKind::FromImport { module, names },
            span,
        })
    }

    fn parse_import_name(&mut self) -> XeResult<(String, String)> {
        let name = self.expect_identifier()?;
        let local = if self.match_token(&TokenKind::As) {
            self.expect_identifier()?
        } else {
            name.clone()
        };
        Ok((name, local))
    }

    fn parse_function_def(&mut self) -> XeResult<Statement> {
        let span = self.current_span();
        self.advance(); // consume 'function'

        let name = self.expect_identifier()?;
        self.expect(&TokenKind::LeftParen)?;

        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            params.push(self.expect_identifier()?);
            while self.match_token(&TokenKind::Comma) {
                params.push(self.expect_identifier()?);
            }
        }
        self.expect(&TokenKind::RightParen)?;
        self.expect(&TokenKind::Colon)?;
        self.expect_newline()?;

        let body = self.parse_block()?;

        Ok(Statement {
            kind: StatementKind::FunctionDef { name, params, body },
            span,
        })
    }

    fn parse_struct_def(&mut self) -> XeResult<Statement> {
        let span = self.current_span();
        self.advance(); // consume 'struct'

        let name = self.expect_identifier()?;
        self.expect(&TokenKind::Colon)?;
        self.expect_newline()?;

        self.expect(&TokenKind::Indent)?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::Dedent) && !self.check(&TokenKind::Eof) {
            if self.match_token(&TokenKind::Newline) {
                continue;
            }
            let field_name = self.expect_identifier()?;
            fields.push(field_name);
            self.expect_statement_end()?;
        }
        self.expect(&TokenKind::Dedent)?;

        Ok(Statement {
            kind: StatementKind::StructDef { name, fields },
            span,
        })
    }

    fn parse_if_like_statement(&mut self, keyword: TokenKind) -> XeResult<Statement> {
        let span = self.current_span();
        self.expect(&keyword)?;

        let condition = self.parse_expression()?;
        self.expect(&TokenKind::Colon)?;
        self.expect_newline()?;

        let then_block = self.parse_block()?;

        let else_block = if self.check(&TokenKind::Elif) {
            let elif_stmt = self.parse_if_like_statement(TokenKind::Elif)?;
            Some(vec![elif_stmt])
        } else if self.check(&TokenKind::Else) {
            self.advance(); // consume 'else'
            self.expect(&TokenKind::Colon)?;
            self.expect_newline()?;
            Some(self.parse_block()?)
        } else {
            None
        };

        Ok(Statement {
            kind: StatementKind::If {
                condition,
                then_block,
                else_block,
            },
            span,
        })
    }

    fn parse_while_statement(&mut self) -> XeResult<Statement> {
        let span = self.current_span();
        self.advance(); // consume 'while'

        let condition = self.parse_expression()?;
        self.expect(&TokenKind::Colon)?;
        self.expect_newline()?;

        let body = self.parse_block()?;

        Ok(Statement {
            kind: StatementKind::While { condition, body },
            span,
        })
    }

    fn parse_repeat_statement(&mut self) -> XeResult<Statement> {
        let span = self.current_span();
        self.advance(); // consume 'repeat'

        let count = self.parse_expression()?;
        self.expect(&TokenKind::Times)?;
        self.expect(&TokenKind::Colon)?;
        self.expect_newline()?;

        let body = self.parse_block()?;

        Ok(Statement {
            kind: StatementKind::Repeat { count, body },
            span,
        })
    }

    fn parse_for_statement(&mut self) -> XeResult<Statement> {
        let span = self.current_span();
        self.advance(); // consume 'for'

        let variable = self.expect_identifier()?;
        self.expect(&TokenKind::In)?;
        let iterable = self.parse_expression()?;
        self.expect(&TokenKind::Colon)?;
        self.expect_newline()?;

        let body = self.parse_block()?;

        Ok(Statement {
            kind: StatementKind::For {
                variable,
                iterable,
                body,
            },
            span,
        })
    }

    fn parse_try_statement(&mut self) -> XeResult<Statement> {
        let span = self.current_span();
        self.advance(); // consume 'try'
        self.expect(&TokenKind::Colon)?;
        self.expect_newline()?;
        let body = self.parse_block()?;

        self.expect(&TokenKind::Catch)?;
        let catch_variable = if self.check(&TokenKind::Colon) {
            None
        } else {
            Some(self.expect_identifier()?)
        };
        self.expect(&TokenKind::Colon)?;
        self.expect_newline()?;
        let handler = self.parse_block()?;

        Ok(Statement {
            kind: StatementKind::Try {
                body,
                catch_variable,
                handler,
            },
            span,
        })
    }

    fn parse_return_statement(&mut self) -> XeResult<Statement> {
        let span = self.current_span();
        self.advance(); // consume 'return'

        let value = if !self.check(&TokenKind::Newline)
            && !self.check(&TokenKind::Dedent)
            && !self.is_at_end()
        {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect_statement_end()?;

        Ok(Statement {
            kind: StatementKind::Return { value },
            span,
        })
    }

    fn parse_block(&mut self) -> XeResult<Vec<Statement>> {
        self.expect(&TokenKind::Indent)?;

        let mut statements = Vec::new();
        while !self.check(&TokenKind::Dedent) && !self.is_at_end() {
            self.skip_newlines();
            if self.check(&TokenKind::Dedent) || self.is_at_end() {
                break;
            }
            statements.push(self.parse_statement()?);
        }

        if self.check(&TokenKind::Dedent) {
            self.advance();
        }

        Ok(statements)
    }

    fn parse_expression(&mut self) -> XeResult<Expression> {
        if self.check(&TokenKind::Lambda) {
            return self.parse_lambda();
        }
        self.parse_binary_expression(0)
    }

    fn parse_lambda(&mut self) -> XeResult<Expression> {
        let span = self.current_span();
        self.advance(); // consume 'lambda'

        let mut params = Vec::new();
        if !self.check(&TokenKind::Colon) {
            params.push(self.expect_identifier()?);
            while self.match_token(&TokenKind::Comma) {
                params.push(self.expect_identifier()?);
            }
        }
        self.expect(&TokenKind::Colon)?;
        let body = self.parse_expression()?;

        Ok(Expression {
            kind: ExpressionKind::Lambda {
                params,
                body: Box::new(body),
            },
            span,
        })
    }

    fn parse_binary_expression(&mut self, min_precedence: u8) -> XeResult<Expression> {
        let mut left = self.parse_prefix_expression()?;

        while let Some((op, token_count)) = self.peek_binary_operator() {
            let precedence = op.precedence();
            if precedence < min_precedence {
                break;
            }

            for _ in 0..token_count {
                self.advance();
            }

            let next_precedence = if op.is_right_associative() {
                precedence
            } else {
                precedence + 1
            };
            let right = self.parse_binary_expression(next_precedence)?;
            let span = left.span.clone();

            left = Expression {
                kind: ExpressionKind::BinaryOp {
                    left: Box::new(left),
                    op,
                    right: Box::new(right),
                },
                span,
            };
        }

        Ok(left)
    }

    fn parse_prefix_expression(&mut self) -> XeResult<Expression> {
        let span = self.current_span();

        // `not` binds looser than comparisons (`not a == b` is `not (a == b)`), and
        // unary minus binds looser than `**` (`-2 ** 2` is `-(2 ** 2)`), as in Python.
        let (op, operand_precedence) = match self.peek_kind() {
            TokenKind::Not => (UnaryOperator::Not, NOT_PRECEDENCE),
            TokenKind::Minus => (UnaryOperator::Negate, NEGATE_PRECEDENCE),
            _ => return self.parse_postfix_expression(),
        };
        self.advance();
        let operand = self.parse_binary_expression(operand_precedence)?;
        Ok(Expression {
            kind: ExpressionKind::UnaryOp {
                op,
                operand: Box::new(operand),
            },
            span,
        })
    }

    fn parse_postfix_expression(&mut self) -> XeResult<Expression> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&TokenKind::LeftParen) {
                let span = expr.span.clone();
                self.advance(); // consume (
                let args = self.parse_arguments()?;
                self.expect(&TokenKind::RightParen)?;
                let kind = match expr.kind {
                    ExpressionKind::Identifier(name) => ExpressionKind::FunctionCall { name, args },
                    ExpressionKind::FieldAccess { object, field } => ExpressionKind::MethodCall {
                        object,
                        method: field,
                        args,
                    },
                    kind => ExpressionKind::Call {
                        callee: Box::new(Expression {
                            kind,
                            span: span.clone(),
                        }),
                        args,
                    },
                };
                expr = Expression { kind, span };
            } else if self.check(&TokenKind::LeftBracket) {
                let span = expr.span.clone();
                self.advance(); // consume [
                let start = if self.check(&TokenKind::Colon) {
                    None
                } else {
                    Some(self.parse_expression()?)
                };
                let kind = if self.match_token(&TokenKind::Colon) {
                    let end = if self.check(&TokenKind::RightBracket) {
                        None
                    } else {
                        Some(Box::new(self.parse_expression()?))
                    };
                    ExpressionKind::Slice {
                        object: Box::new(expr),
                        start: start.map(Box::new),
                        end,
                    }
                } else {
                    match start {
                        Some(index) => ExpressionKind::Index {
                            object: Box::new(expr),
                            index: Box::new(index),
                        },
                        None => {
                            return Err(XeError::new(
                                XeErrorKind::ExpectedExpression,
                                Some(self.current_span()),
                            ))
                        }
                    }
                };
                self.expect(&TokenKind::RightBracket)?;
                expr = Expression { kind, span };
            } else if self.check(&TokenKind::Dot) {
                let span = expr.span.clone();
                self.advance(); // consume .
                let field = self.expect_identifier()?;
                expr = Expression {
                    kind: ExpressionKind::FieldAccess {
                        object: Box::new(expr),
                        field,
                    },
                    span,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> XeResult<Expression> {
        let span = self.current_span();

        let kind = match self.peek_kind() {
            TokenKind::Number(n) => ExpressionKind::Number(*n),
            TokenKind::String(s) => ExpressionKind::String(s.clone()),
            TokenKind::True => ExpressionKind::Boolean(true),
            TokenKind::False => ExpressionKind::Boolean(false),
            TokenKind::None => ExpressionKind::None,
            TokenKind::Identifier(name) => ExpressionKind::Identifier(name.clone()),
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RightParen)?;
                return Ok(expr);
            }
            TokenKind::LeftBracket => {
                self.advance();
                let elements = self.parse_list_elements()?;
                self.expect(&TokenKind::RightBracket)?;
                return Ok(Expression {
                    kind: ExpressionKind::List(elements),
                    span,
                });
            }
            TokenKind::LeftBrace => {
                self.advance();
                let entries = self.parse_map_entries()?;
                self.expect(&TokenKind::RightBrace)?;
                return Ok(Expression {
                    kind: ExpressionKind::Map(entries),
                    span,
                });
            }
            _ => return Err(XeError::new(XeErrorKind::ExpectedExpression, Some(span))),
        };
        self.advance();
        Ok(Expression { kind, span })
    }

    fn parse_arguments(&mut self) -> XeResult<Vec<Expression>> {
        let mut args = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            args.push(self.parse_expression()?);
            while self.match_token(&TokenKind::Comma) {
                if self.check(&TokenKind::RightParen) {
                    break;
                }
                args.push(self.parse_expression()?);
            }
        }
        Ok(args)
    }

    fn parse_list_elements(&mut self) -> XeResult<Vec<Expression>> {
        let mut elements = Vec::new();
        if !self.check(&TokenKind::RightBracket) {
            elements.push(self.parse_expression()?);
            while self.match_token(&TokenKind::Comma) {
                if self.check(&TokenKind::RightBracket) {
                    break;
                }
                elements.push(self.parse_expression()?);
            }
        }
        Ok(elements)
    }

    fn parse_map_entries(&mut self) -> XeResult<Vec<(Expression, Expression)>> {
        let mut entries = Vec::new();
        if !self.check(&TokenKind::RightBrace) {
            let key = self.parse_expression()?;
            self.expect(&TokenKind::Colon)?;
            let value = self.parse_expression()?;
            entries.push((key, value));

            while self.match_token(&TokenKind::Comma) {
                if self.check(&TokenKind::RightBrace) {
                    break;
                }
                let key = self.parse_expression()?;
                self.expect(&TokenKind::Colon)?;
                let value = self.parse_expression()?;
                entries.push((key, value));
            }
        }
        Ok(entries)
    }

    fn parse_module_path(&mut self) -> XeResult<ModulePath> {
        let mut segments = vec![self.expect_identifier()?];
        while self.match_token(&TokenKind::Dot) {
            segments.push(self.expect_identifier()?);
        }
        Ok(ModulePath { segments })
    }

    /// The binary operator at the current position and how many tokens it spans.
    fn peek_binary_operator(&self) -> Option<(BinaryOperator, usize)> {
        let op = match self.peek_kind() {
            TokenKind::Plus => BinaryOperator::Add,
            TokenKind::Minus => BinaryOperator::Subtract,
            TokenKind::Star => BinaryOperator::Multiply,
            TokenKind::Slash => BinaryOperator::Divide,
            TokenKind::SlashSlash => BinaryOperator::FloorDivide,
            TokenKind::Percent => BinaryOperator::Modulo,
            TokenKind::StarStar => BinaryOperator::Power,
            TokenKind::EqualEqual => BinaryOperator::Equal,
            TokenKind::NotEqual => BinaryOperator::NotEqual,
            TokenKind::Less => BinaryOperator::Less,
            TokenKind::Greater => BinaryOperator::Greater,
            TokenKind::LessEqual => BinaryOperator::LessEqual,
            TokenKind::GreaterEqual => BinaryOperator::GreaterEqual,
            TokenKind::In => BinaryOperator::In,
            TokenKind::Not if self.peek_next_kind() == Some(&TokenKind::In) => {
                return Some((BinaryOperator::NotIn, 2));
            }
            TokenKind::And => BinaryOperator::And,
            TokenKind::Or => BinaryOperator::Or,
            _ => return None,
        };
        Some((op, 1))
    }

    // Helper methods

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn peek_kind(&self) -> &TokenKind {
        self.peek().map(|t| &t.kind).unwrap_or(&TokenKind::Eof)
    }

    fn peek_next_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.pos + 1).map(|t| &t.kind)
    }

    fn current_span(&self) -> Span {
        self.peek()
            .map(|t| t.span.clone())
            .unwrap_or_else(|| Span::new(1, 1))
    }

    fn advance(&mut self) -> Option<&Token> {
        if !self.is_at_end() {
            self.pos += 1;
        }
        self.tokens.get(self.pos - 1)
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek_kind(), TokenKind::Eof)
    }

    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(self.peek_kind()) == std::mem::discriminant(kind)
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind) -> XeResult<()> {
        if self.check(kind) {
            self.advance();
            Ok(())
        } else {
            Err(XeError::new(
                XeErrorKind::ExpectedToken(format_token_kind(kind).to_string()),
                Some(self.current_span()),
            ))
        }
    }

    fn expect_identifier(&mut self) -> XeResult<String> {
        if let TokenKind::Identifier(name) = self.peek_kind() {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(XeError::new(
                XeErrorKind::ExpectedIdentifier,
                Some(self.current_span()),
            ))
        }
    }

    fn expect_newline(&mut self) -> XeResult<()> {
        if self.check(&TokenKind::Newline) {
            self.advance();
            self.skip_newlines();
            Ok(())
        } else if self.is_at_end() {
            Ok(())
        } else {
            Err(XeError::new(
                XeErrorKind::ExpectedToken("newline".to_string()),
                Some(self.current_span()),
            ))
        }
    }

    fn expect_statement_end(&mut self) -> XeResult<()> {
        if self.check(&TokenKind::Newline) {
            self.advance();
            self.skip_newlines();
            Ok(())
        } else if self.is_at_end() || self.check(&TokenKind::Dedent) {
            Ok(())
        } else {
            Err(XeError::new(
                XeErrorKind::ExpectedToken("end of statement".to_string()),
                Some(self.current_span()),
            ))
        }
    }

    fn skip_newlines(&mut self) {
        while self.check(&TokenKind::Newline) {
            self.advance();
        }
    }
}

fn augmented_operator(kind: &TokenKind) -> Option<BinaryOperator> {
    Some(match kind {
        TokenKind::PlusEqual => BinaryOperator::Add,
        TokenKind::MinusEqual => BinaryOperator::Subtract,
        TokenKind::StarEqual => BinaryOperator::Multiply,
        TokenKind::SlashEqual => BinaryOperator::Divide,
        TokenKind::SlashSlashEqual => BinaryOperator::FloorDivide,
        TokenKind::PercentEqual => BinaryOperator::Modulo,
        TokenKind::StarStarEqual => BinaryOperator::Power,
        _ => return None,
    })
}

fn format_token_kind(kind: &TokenKind) -> &'static str {
    match kind {
        TokenKind::Number(_) => "number",
        TokenKind::String(_) => "string",
        TokenKind::True => "true",
        TokenKind::False => "false",
        TokenKind::Identifier(_) => "identifier",
        TokenKind::If => "if",
        TokenKind::Else => "else",
        TokenKind::Elif => "elif",
        TokenKind::Function => "fun",
        TokenKind::While => "while",
        TokenKind::For => "for",
        TokenKind::In => "in",
        TokenKind::Repeat => "repeat",
        TokenKind::Times => "times",
        TokenKind::And => "and",
        TokenKind::Or => "or",
        TokenKind::Not => "not",
        TokenKind::Return => "return",
        TokenKind::Break => "break",
        TokenKind::Continue => "continue",
        TokenKind::Import => "import",
        TokenKind::From => "from",
        TokenKind::Struct => "struct",
        TokenKind::Global => "global",
        TokenKind::None => "none",
        TokenKind::Pass => "pass",
        TokenKind::Try => "try",
        TokenKind::Catch => "catch",
        TokenKind::Lambda => "lambda",
        TokenKind::As => "as",
        TokenKind::Plus => "+",
        TokenKind::Minus => "-",
        TokenKind::Star => "*",
        TokenKind::Slash => "/",
        TokenKind::Percent => "%",
        TokenKind::StarStar => "**",
        TokenKind::SlashSlash => "//",
        TokenKind::PlusEqual => "+=",
        TokenKind::MinusEqual => "-=",
        TokenKind::StarEqual => "*=",
        TokenKind::SlashEqual => "/=",
        TokenKind::SlashSlashEqual => "//=",
        TokenKind::PercentEqual => "%=",
        TokenKind::StarStarEqual => "**=",
        TokenKind::Equal => "=",
        TokenKind::EqualEqual => "==",
        TokenKind::NotEqual => "!=",
        TokenKind::Less => "<",
        TokenKind::Greater => ">",
        TokenKind::LessEqual => "<=",
        TokenKind::GreaterEqual => ">=",
        TokenKind::LeftParen => "(",
        TokenKind::RightParen => ")",
        TokenKind::LeftBracket => "[",
        TokenKind::RightBracket => "]",
        TokenKind::LeftBrace => "{",
        TokenKind::RightBrace => "}",
        TokenKind::Colon => ":",
        TokenKind::Comma => ",",
        TokenKind::Dot => ".",
        TokenKind::Newline => "newline",
        TokenKind::Indent => "indent",
        TokenKind::Dedent => "dedent",
        TokenKind::Eof => "end of file",
    }
}
