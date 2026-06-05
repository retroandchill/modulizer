use crate::preprocessor::Lexeme;

pub struct Lexer<'a> {
    source: &'a str,
    position: usize,
}

impl<'a> Lexer<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        Lexer {
            source,
            position: 0,
        }
    }

    pub fn next_lexeme(&mut self) -> Option<Lexeme> {
        let current = self.peek_char()?;

        // We want to skip \r characters, as they are often used in Windows line endings
        if current == '\r' {
            self.advance_char();
            return self.next_lexeme();
        }

        if current == '\n' {
            self.advance_char();
            return Some(Lexeme::NewLine);
        }

        if current.is_whitespace() {
            return Some(Lexeme::Whitespace(self.take_while(|ch| {
                ch.is_whitespace() && ch != '\n'
            })));
        }

        if current == '/' {
            if self.starts_with("//") {
                return Some(Lexeme::LineComment(self.take_until_newline()))
            }

            if self.starts_with("/*") {
                return Some(Lexeme::BlockComment(self.take_block_comment()))
            }

            self.advance_char();
            return Some(Lexeme::Other(current.to_string()));
        }

        if current == '"' || self.starts_string_literal_prefix() {
            return Some(Lexeme::StringLiteral(self.take_string_literal()));
        }

        if Self::is_identifier_start(current) {
            return Some(Lexeme::Identifier(self.take_while(Self::is_identifier_continue)));
        }

        if current.is_ascii_digit() {
            return Some(Lexeme::IntegerLiteral(self.take_integer_literal()));
        }

        match current {
            '#' => {
                self.advance_char();
                Some(Lexeme::Hash)
            }
            '\\' => {
                self.advance_char();
                Some(Lexeme::Backslash)
            }
            '(' => {
                self.advance_char();
                Some(Lexeme::LParen)
            }
            ')' => {
                self.advance_char();
                Some(Lexeme::RParen)
            }
            '{' => {
                self.advance_char();
                Some(Lexeme::LBrace)
            }
            '}' => {
                self.advance_char();
                Some(Lexeme::RBrace)
            }
            ',' => {
                self.advance_char();
                Some(Lexeme::Comma)
            }
            ';' => {
                self.advance_char();
                Some(Lexeme::Semicolon)
            }
            ':' if self.starts_with("::") => {
                self.position += 2;
                Some(Lexeme::DoubleColon)
            }
            ':' => {
                self.advance_char();
                Some(Lexeme::Colon)
            }
            '<' if self.starts_with("<=") => {
                self.position += 2;
                Some(Lexeme::LessEqual)
            }
            '<' => {
                self.advance_char();
                Some(Lexeme::Less)
            }
            '>' if self.starts_with(">=") => {
                self.position += 2;
                Some(Lexeme::GreaterEqual)
            }
            '>' => {
                self.advance_char();
                Some(Lexeme::Greater)
            }
            '=' if self.starts_with("==") => {
                self.position += 2;
                Some(Lexeme::DoubleEqual)
            }
            '!' if self.starts_with("!=") => {
                self.position += 2;
                Some(Lexeme::NotEqual)
            }
            '=' => {
                self.advance_char();
                Some(Lexeme::Equal)
            }
            _ => Some(Lexeme::Other(self.advance_char().to_string())),
        }
     }

    fn peek_char(&self) -> Option<char> {
        self.source[self.position..].chars().next()
    }

    fn advance_char(&mut self) -> char {
        let ch = self.peek_char().expect("advanced past end of source");
        self.position += ch.len_utf8();
        ch

    }

    fn starts_with(&self, value: &str) -> bool {
        self.source[self.position..].starts_with(value)
    }

    fn take_while(&mut self, predicate: impl Fn(char) -> bool) -> String {
        let start = self.position;

        while let Some(ch) = self.peek_char() {
            if !predicate(ch) {
                break;
            }

            self.advance_char();
        }

        self.source[start..self.position].to_string()
    }

    fn take_until_newline(&mut self) -> String {
        self.take_while(|ch| ch != '\n')
    }

    fn take_block_comment(&mut self) -> String {
        let start = self.position;
        self.position += 2;

        while self.position < self.source.len() {
            if self.starts_with("*/") {
                self.position += 2;
                break;
            }

            self.advance_char();
        }

        self.source[start..self.position].to_string()
    }

    fn starts_string_literal_prefix(&self) -> bool {
        self.starts_with("u8\"")
            || self.starts_with("u\"")
            || self.starts_with("U\"")
            || self.starts_with("L\"")
            || self.starts_with("R\"")
            || self.starts_with("u8R\"")
            || self.starts_with("uR\"")
            || self.starts_with("UR\"")
            || self.starts_with("LR\"")
    }

    fn take_string_literal(&mut self) -> String {
        if let Some(raw_start) = self.source[self.position..].find("R\"") {
            let raw_position = self.position + raw_start;

            if self.source[self.position..raw_position]
                .chars()
                .all(|ch| matches!(ch, 'u' | 'U' | 'L' | '8'))
            {
                self.position = raw_position + 2;
                return self.take_raw_string_literal_from(raw_position);
            }
        }

        while self.peek_char() != Some('"') {
            self.advance_char();
        }

        self.take_quoted_literal('"')
    }

    fn take_quoted_literal(&mut self, quote: char) -> String {
        let start = self.position;

        self.advance_char();

        while let Some(ch) = self.peek_char() {
            self.advance_char();

            if ch == '\\' {
                if self.peek_char().is_some() {
                    self.advance_char();
                }

                continue;
            }

            if ch == quote {
                break;
            }
        }

        self.source[start..self.position].to_string()
    }

    fn take_raw_string_literal_from(&mut self, start: usize) -> String {
        let delimiter_start = self.position;

        while let Some(ch) = self.peek_char() {
            self.advance_char();

            if ch == '(' {
                break;
            }
        }

        let delimiter = &self.source[delimiter_start..self.position - 1];
        let terminator = format!("){}\"", delimiter);

        if let Some(end_offset) = self.source[self.position..].find(&terminator) {
            self.position += end_offset + terminator.len();
        } else {
            self.position = self.source.len();
        }

        self.source[start..self.position].to_string()
    }

    fn take_integer_literal(&mut self) -> String {
        self.take_while(|ch| ch.is_ascii_alphanumeric() || ch == '\'' || ch == '_')
    }

    fn is_identifier_start(ch: char) -> bool {
        ch == '_' || ch.is_ascii_alphabetic()
    }

    fn is_identifier_continue(ch: char) -> bool {
        ch == '_' || ch.is_ascii_alphanumeric()
    }
}