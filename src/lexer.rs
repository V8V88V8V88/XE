use crate::error::{Span, XeError, XeErrorKind, XeResult};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals
    Number(f64),
    String(String),
    True,
    False,

    // Identifiers and keywords
    Identifier(String),
    If,
    Else,
    Function,
    Repeat,
    Times,
    And,
    Or,
    Not,
    Return,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Equal,
    EqualEqual,
    NotEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,

    // Delimiters
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Colon,
    Comma,

    // Structure
    Newline,
    Indent,
    Dedent,
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Self {
            kind,
            span: Span::new(line, column),
        }
    }
}

pub struct Lexer {
    source: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    indent_stack: Vec<usize>,
    pending_tokens: Vec<Token>,
    at_line_start: bool,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            source: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            indent_stack: vec![0],
            pending_tokens: Vec::new(),
            at_line_start: true,
        }
    }

    pub fn tokenize(&mut self) -> XeResult<Vec<Token>> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token()?;
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }

        Ok(tokens)
    }

    fn next_token(&mut self) -> XeResult<Token> {
        // Return pending tokens first (dedents)
        if let Some(token) = self.pending_tokens.pop() {
            return Ok(token);
        }

        // Handle indentation at line start
        if self.at_line_start {
            self.at_line_start = false;
            if let Some(token) = self.handle_indentation()? {
                return Ok(token);
            }
        }

        self.skip_whitespace_same_line();

        if self.is_at_end() {
            // Emit remaining dedents
            while self.indent_stack.len() > 1 {
                self.indent_stack.pop();
                self.pending_tokens.push(Token::new(TokenKind::Dedent, self.line, self.column));
            }
            if let Some(token) = self.pending_tokens.pop() {
                return Ok(token);
            }
            return Ok(Token::new(TokenKind::Eof, self.line, self.column));
        }

        let c = self.peek();

        // Handle comments
        if c == '#' {
            self.skip_comment();
            return self.next_token();
        }

        // Handle newlines
        if c == '\n' {
            let token = Token::new(TokenKind::Newline, self.line, self.column);
            self.advance();
            self.line += 1;
            self.column = 1;
            self.at_line_start = true;
            return Ok(token);
        }

        // Handle carriage return
        if c == '\r' {
            self.advance();
            if self.peek() == '\n' {
                self.advance();
            }
            let token = Token::new(TokenKind::Newline, self.line, self.column);
            self.line += 1;
            self.column = 1;
            self.at_line_start = true;
            return Ok(token);
        }

        let start_column = self.column;

        // String literals
        if c == '"' {
            return self.scan_string();
        }

        // Numbers
        if c.is_ascii_digit() {
            return self.scan_number();
        }

        // Identifiers and keywords
        if c.is_alphabetic() || c == '_' {
            return self.scan_identifier();
        }

        // Operators and delimiters
        self.scan_operator_or_delimiter(start_column)
    }

    fn handle_indentation(&mut self) -> XeResult<Option<Token>> {
        let mut indent = 0;
        while self.peek() == ' ' {
            indent += 1;
            self.advance();
        }
        // Tabs count as 4 spaces
        while self.peek() == '\t' {
            indent += 4;
            self.advance();
        }

        // Skip empty lines and comment-only lines
        if self.peek() == '\n' || self.peek() == '\r' || self.peek() == '#' || self.is_at_end() {
            return Ok(None);
        }

        let current_indent = *self.indent_stack.last().unwrap();

        if indent > current_indent {
            self.indent_stack.push(indent);
            return Ok(Some(Token::new(TokenKind::Indent, self.line, 1)));
        } else if indent < current_indent {
            while self.indent_stack.len() > 1 && *self.indent_stack.last().unwrap() > indent {
                self.indent_stack.pop();
                self.pending_tokens.push(Token::new(TokenKind::Dedent, self.line, 1));
            }
            if *self.indent_stack.last().unwrap() != indent {
                return Err(XeError::new(
                    XeErrorKind::InvalidIndentation,
                    Some(Span::new(self.line, 1)),
                ));
            }
            return Ok(self.pending_tokens.pop());
        }

        Ok(None)
    }

    fn scan_string(&mut self) -> XeResult<Token> {
        let start_line = self.line;
        let start_column = self.column;
        self.advance(); // consume opening quote

        let mut value = String::new();
        while !self.is_at_end() && self.peek() != '"' {
            if self.peek() == '\n' {
                return Err(XeError::new(
                    XeErrorKind::UnterminatedString,
                    Some(Span::new(start_line, start_column)),
                ));
            }
            if self.peek() == '\\' {
                self.advance();
                match self.peek() {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    _ => value.push(self.peek()),
                }
            } else {
                value.push(self.peek());
            }
            self.advance();
        }

        if self.is_at_end() {
            return Err(XeError::new(
                XeErrorKind::UnterminatedString,
                Some(Span::new(start_line, start_column)),
            ));
        }

        self.advance(); // consume closing quote
        Ok(Token::new(TokenKind::String(value), start_line, start_column))
    }

    fn scan_number(&mut self) -> XeResult<Token> {
        let start_column = self.column;
        let mut num_str = String::new();

        while !self.is_at_end() && (self.peek().is_ascii_digit() || self.peek() == '.') {
            num_str.push(self.peek());
            self.advance();
        }

        match num_str.parse::<f64>() {
            Ok(n) => Ok(Token::new(TokenKind::Number(n), self.line, start_column)),
            Err(_) => Err(XeError::new(
                XeErrorKind::InvalidNumber(num_str),
                Some(Span::new(self.line, start_column)),
            )),
        }
    }

    fn scan_identifier(&mut self) -> XeResult<Token> {
        let start_column = self.column;
        let mut ident = String::new();

        while !self.is_at_end() && (self.peek().is_alphanumeric() || self.peek() == '_') {
            ident.push(self.peek());
            self.advance();
        }

        let kind = match ident.as_str() {
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "function" => TokenKind::Function,
            "repeat" => TokenKind::Repeat,
            "times" => TokenKind::Times,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "return" => TokenKind::Return,
            _ => TokenKind::Identifier(ident),
        };

        Ok(Token::new(kind, self.line, start_column))
    }

    fn scan_operator_or_delimiter(&mut self, start_column: usize) -> XeResult<Token> {
        let c = self.peek();
        self.advance();

        let kind = match c {
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '(' => TokenKind::LeftParen,
            ')' => TokenKind::RightParen,
            '[' => TokenKind::LeftBracket,
            ']' => TokenKind::RightBracket,
            ':' => TokenKind::Colon,
            ',' => TokenKind::Comma,
            '=' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::EqualEqual
                } else {
                    TokenKind::Equal
                }
            }
            '!' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::NotEqual
                } else {
                    return Err(XeError::new(
                        XeErrorKind::UnexpectedCharacter(c),
                        Some(Span::new(self.line, start_column)),
                    ));
                }
            }
            '<' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                }
            }
            '>' => {
                if self.peek() == '=' {
                    self.advance();
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }
            _ => {
                return Err(XeError::new(
                    XeErrorKind::UnexpectedCharacter(c),
                    Some(Span::new(self.line, start_column)),
                ));
            }
        };

        Ok(Token::new(kind, self.line, start_column))
    }

    fn skip_whitespace_same_line(&mut self) {
        while !self.is_at_end() && (self.peek() == ' ' || self.peek() == '\t') {
            self.advance();
        }
    }

    fn skip_comment(&mut self) {
        while !self.is_at_end() && self.peek() != '\n' {
            self.advance();
        }
    }

    fn peek(&self) -> char {
        self.source.get(self.pos).copied().unwrap_or('\0')
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.pos += 1;
            self.column += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.source.len()
    }
}
