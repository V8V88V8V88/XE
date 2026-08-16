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

        if self.check(&TokenKind::Import) {
            return self.parse_import_statement();
        }

        if self.check(&TokenKind::From) {
            return self.parse_from_import_statement();
        }

        // Function definition
        if self.check(&TokenKind::Function) {
            return self.parse_function_def();
        }

        // If statement
        if self.check(&TokenKind::If) {
            return self.parse_if_statement();
        }

        // While loop
        if self.check(&TokenKind::While) {
            return self.parse_while_statement();
        }

        // Repeat loop
        if self.check(&TokenKind::Repeat) {
            return self.parse_repeat_statement();
        }

        // For loop
        if self.check(&TokenKind::For) {
            return self.parse_for_statement();
        }

        // Return statement
        if self.check(&TokenKind::Return) {
            return self.parse_return_statement();
        }

        if self.check(&TokenKind::Break) {
            self.advance();
            self.expect_statement_end()?;
            return Ok(Statement {
                kind: StatementKind::Break,
                span,
            });
        }

        if self.check(&TokenKind::Continue) {
            self.advance();
            self.expect_statement_end()?;
            return Ok(Statement {
                kind: StatementKind::Continue,
                span,
            });
        }

        // Assignment or expression
        if let TokenKind::Identifier(name) = self.peek_kind() {
            let name = name.clone();
            if self.peek_next_kind() == Some(&TokenKind::Equal) {
                self.advance(); // consume identifier
                self.advance(); // consume =
                let value = self.parse_expression()?;
                self.expect_statement_end()?;
                return Ok(Statement {
                    kind: StatementKind::Assignment { name, value },
                    span,
                });
            }
        }

        // Expression statement
        let expr = self.parse_expression()?;
        self.expect_statement_end()?;
        Ok(Statement {
            kind: StatementKind::Expression(expr),
            span,
        })
    }

    fn parse_import_statement(&mut self) -> XeResult<Statement> {
        let span = self.current_span();
        self.advance(); // consume 'import'
        let module = self.parse_module_path()?;
        self.expect_statement_end()?;
        Ok(Statement {
            kind: StatementKind::Import { module },
            span,
        })
    }

    fn parse_from_import_statement(&mut self) -> XeResult<Statement> {
        let span = self.current_span();
        self.advance(); // consume 'from'
        let module = self.parse_module_path()?;
        self.expect(&TokenKind::Import)?;

        let mut names = vec![self.expect_identifier()?];
        while self.match_token(&TokenKind::Comma) {
            names.push(self.expect_identifier()?);
        }

        self.expect_statement_end()?;
        Ok(Statement {
            kind: StatementKind::FromImport { module, names },
            span,
        })
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

    fn parse_if_statement(&mut self) -> XeResult<Statement> {
        self.parse_if_like_statement(TokenKind::If)
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

    fn parse_return_statement(&mut self) -> XeResult<Statement> {
        let span = self.current_span();
        self.advance(); // consume 'return'

        let value = if !self.check(&TokenKind::Newline) && !self.is_at_end() {
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
        self.parse_binary_expression(0)
    }

    fn parse_binary_expression(&mut self, min_precedence: u8) -> XeResult<Expression> {
        let mut left = self.parse_unary_expression()?;

        while let Some(op) = self.peek_binary_operator() {
            let precedence = op.precedence();
            if precedence < min_precedence {
                break;
            }

            self.advance(); // consume operator

            let right = self.parse_binary_expression(precedence + 1)?;
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

    fn parse_unary_expression(&mut self) -> XeResult<Expression> {
        let span = self.current_span();

        if self.check(&TokenKind::Minus) {
            self.advance();
            let operand = self.parse_unary_expression()?;
            return Ok(Expression {
                kind: ExpressionKind::UnaryOp {
                    op: UnaryOperator::Negate,
                    operand: Box::new(operand),
                },
                span,
            });
        }

        if self.check(&TokenKind::Not) {
            self.advance();
            let operand = self.parse_unary_expression()?;
            return Ok(Expression {
                kind: ExpressionKind::UnaryOp {
                    op: UnaryOperator::Not,
                    operand: Box::new(operand),
                },
                span,
            });
        }

        self.parse_postfix_expression()
    }

    fn parse_postfix_expression(&mut self) -> XeResult<Expression> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.check(&TokenKind::LeftParen) {
                // Function call
                if let ExpressionKind::Identifier(name) = &expr.kind {
                    let name = name.clone();
                    let span = expr.span.clone();
                    self.advance(); // consume (
                    let args = self.parse_arguments()?;
                    self.expect(&TokenKind::RightParen)?;
                    expr = Expression {
                        kind: ExpressionKind::FunctionCall { name, args },
                        span,
                    };
                } else {
                    break;
                }
            } else if self.check(&TokenKind::LeftBracket) {
                // Index access
                let span = expr.span.clone();
                self.advance(); // consume [
                let index = self.parse_expression()?;
                self.expect(&TokenKind::RightBracket)?;
                expr = Expression {
                    kind: ExpressionKind::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
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

        match self.peek_kind() {
            TokenKind::Number(n) => {
                let n = *n;
                self.advance();
                Ok(Expression {
                    kind: ExpressionKind::Number(n),
                    span,
                })
            }
            TokenKind::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression {
                    kind: ExpressionKind::String(s),
                    span,
                })
            }
            TokenKind::True => {
                self.advance();
                Ok(Expression {
                    kind: ExpressionKind::Boolean(true),
                    span,
                })
            }
            TokenKind::False => {
                self.advance();
                Ok(Expression {
                    kind: ExpressionKind::Boolean(false),
                    span,
                })
            }
            TokenKind::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expression {
                    kind: ExpressionKind::Identifier(name),
                    span,
                })
            }
            TokenKind::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&TokenKind::RightParen)?;
                Ok(expr)
            }
            TokenKind::LeftBracket => {
                self.advance();
                let elements = self.parse_list_elements()?;
                self.expect(&TokenKind::RightBracket)?;
                Ok(Expression {
                    kind: ExpressionKind::List(elements),
                    span,
                })
            }
            _ => Err(XeError::new(XeErrorKind::ExpectedExpression, Some(span))),
        }
    }

    fn parse_arguments(&mut self) -> XeResult<Vec<Expression>> {
        let mut args = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            args.push(self.parse_expression()?);
            while self.match_token(&TokenKind::Comma) {
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
                elements.push(self.parse_expression()?);
            }
        }
        Ok(elements)
    }

    fn parse_module_path(&mut self) -> XeResult<ModulePath> {
        let mut segments = vec![self.expect_identifier()?];
        while self.match_token(&TokenKind::Dot) {
            segments.push(self.expect_identifier()?);
        }
        Ok(ModulePath { segments })
    }

    fn peek_binary_operator(&self) -> Option<BinaryOperator> {
        match self.peek_kind() {
            TokenKind::Plus => Some(BinaryOperator::Add),
            TokenKind::Minus => Some(BinaryOperator::Subtract),
            TokenKind::Star => Some(BinaryOperator::Multiply),
            TokenKind::Slash => Some(BinaryOperator::Divide),
            TokenKind::Percent => Some(BinaryOperator::Modulo),
            TokenKind::EqualEqual => Some(BinaryOperator::Equal),
            TokenKind::NotEqual => Some(BinaryOperator::NotEqual),
            TokenKind::Less => Some(BinaryOperator::Less),
            TokenKind::Greater => Some(BinaryOperator::Greater),
            TokenKind::LessEqual => Some(BinaryOperator::LessEqual),
            TokenKind::GreaterEqual => Some(BinaryOperator::GreaterEqual),
            TokenKind::And => Some(BinaryOperator::And),
            TokenKind::Or => Some(BinaryOperator::Or),
            _ => None,
        }
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
        TokenKind::Plus => "+",
        TokenKind::Minus => "-",
        TokenKind::Star => "*",
        TokenKind::Slash => "/",
        TokenKind::Percent => "%",
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
        TokenKind::Colon => ":",
        TokenKind::Comma => ",",
        TokenKind::Dot => ".",
        TokenKind::Newline => "newline",
        TokenKind::Indent => "indent",
        TokenKind::Dedent => "dedent",
        TokenKind::Eof => "end of file",
    }
}

