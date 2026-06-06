use crate::preprocessor::lexer::Lexer;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lexeme {
    Identifier(String),
    
    StringLiteral(String),
    CharacterLiteral(String),
    IntegerLiteral(String),

    Whitespace(String),
    NewLine,
    LineComment(String),
    BlockComment(String),
    Slash,
    
    Hash,
    
    LParen,
    RParen,
    LBrace,
    RBrace,
    
    Comma,
    Semicolon,
    
    Colon,
    DoubleColon,
    
    Less,
    Greater,
    
    Equal,
    DoubleEqual,
    GreaterEqual,
    LessEqual,
    NotEqual,

    Not,
    And,
    Or,
    
    Pack,
    
    Other(String)
}

pub fn lex(source: &str) -> Vec<Lexeme> {
    let mut lexer = Lexer::new(source);
    let mut lexemes = Vec::new();
    
    while let Some(lexeme) = lexer.next_lexeme() {
        lexemes.push(lexeme);
    }
    
    lexemes
}

impl Lexeme {
    pub fn as_source(&self) -> &str {
        match self {
            Lexeme::Identifier(value)
            | Lexeme::StringLiteral(value)
            | Lexeme::CharacterLiteral(value)
            | Lexeme::IntegerLiteral(value)
            | Lexeme::Whitespace(value)
            | Lexeme::LineComment(value)
            | Lexeme::BlockComment(value)
            | Lexeme::Other(value) => value,
            Lexeme::NewLine => "\n",
            Lexeme::Slash => "\\",
            Lexeme::Hash => "#",
            Lexeme::LParen => "(",
            Lexeme::RParen => ")",
            Lexeme::LBrace => "{",
            Lexeme::RBrace => "}",
            Lexeme::Comma => ",",
            Lexeme::Semicolon => ";",
            Lexeme::Colon => ":",
            Lexeme::DoubleColon => "::",
            Lexeme::Less => "<",
            Lexeme::Greater => ">",
            Lexeme::Equal => "=",
            Lexeme::DoubleEqual => "==",
            Lexeme::GreaterEqual => ">=",
            Lexeme::LessEqual => "<=",
            Lexeme::NotEqual => "!=",
            Lexeme::And => "&&",
            Lexeme::Or => "||",
            Lexeme::Not => "!",
            Lexeme::Pack => "...",
        }
    }
}