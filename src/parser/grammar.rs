use logos::Logos;
use std::fmt;
use std::fmt::Formatter;

#[derive(Logos, Clone, Debug, PartialEq)]
pub enum Token {
    #[regex(r"\r\n|\n|\r")]
    NewLine,

    #[regex(r"[ \t\f]+", |lex| lex.slice().to_string())]
    Whitespace(String),

    #[regex(r"//[^\r\n]*", allow_greedy = true)]
    LineComment,

    #[regex(r"/\*([^*]|\*[^/])*\*/")]
    BlockComment,

    #[token("#")]
    Hash,

    #[token("##")]
    HashHash,

    #[token("namespace")]
    Namespace,

    #[token("inline")]
    Inline,

    #[token("using")]
    Using,

    #[token("typedef")]
    Typedef,

    #[token("class")]
    Class,

    #[token("struct")]
    Struct,

    #[token("enum")]
    Enum,

    #[token("union")]
    Union,

    #[token("template")]
    Template,

    #[token("typename")]
    Typename,

    #[token("concept")]
    Concept,

    #[token("requires")]
    Requires,

    #[token("friend")]
    Friend,

    #[token("extern")]
    Extern,

    #[token("static")]
    Static,

    #[token("explicit")]
    Explicit,

    #[token("const")]
    Const,

    #[token("mutable")]
    Mutable,

    #[token("volatile")]
    Volatile,

    #[token("constexpr")]
    Constexpr,

    #[token("consteval")]
    Consteval,

    #[token("constinit")]
    Constinit,

    #[token("auto")]
    Auto,

    #[token("operator")]
    Operator,

    #[token("public")]
    Public,

    #[token("protected")]
    Protected,

    #[token("private")]
    Private,

    #[regex(r#"(u8|u|U|L)?""#, |lex| lex.slice().to_string())]
    StringLiteral(String),

    #[regex(r"(u8|u|U|L)?'([^'\\]|\\.|\\\r?\n)*'", |lex| lex.slice().to_string())]
    CharacterLiteral(String),

    #[regex(r"([0-9][0-9']*)(\.[0-9']*)?([eEpP][+-]?[0-9']+)?[a-zA-Z_0-9]*", |lex| lex.slice().to_string())]
    NumberLiteral(String),

    // Identifiers.
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),

    // Multi-character operators and punctuation.
    #[token("::")]
    DoubleColon,

    #[token("...")]
    Ellipsis,

    #[token("->")]
    Arrow,

    #[token("->*")]
    ArrowStar,

    #[token(".*")]
    DotStar,

    #[token("<=>")]
    Spaceship,

    #[token("==")]
    EqualEqual,

    #[token("!=")]
    NotEqual,

    #[token("<=")]
    LessEqual,

    #[token(">=")]
    GreaterEqual,

    #[token("&&")]
    AndAnd,

    #[token("||")]
    OrOr,

    #[token("++")]
    PlusPlus,

    #[token("--")]
    MinusMinus,

    #[token("+=")]
    PlusEqual,

    #[token("-=")]
    MinusEqual,

    #[token("*=")]
    StarEqual,

    #[token("/=")]
    SlashEqual,

    #[token("%=")]
    PercentEqual,

    #[token("&=")]
    AmpEqual,

    #[token("|=")]
    PipeEqual,

    #[token("^=")]
    CaretEqual,

    #[token("<<")]
    ShiftLeft,

    #[token(">>")]
    ShiftRight,

    #[token("<<=")]
    ShiftLeftEqual,

    #[token(">>=")]
    ShiftRightEqual,

    // Single-character punctuation.
    #[token("(")]
    LParen,

    #[token(")")]
    RParen,

    #[token("{")]
    LBrace,

    #[token("}")]
    RBrace,

    #[token("[")]
    LBracket,

    #[token("]")]
    RBracket,

    #[token("<")]
    Less,

    #[token(">")]
    Greater,

    #[token(",")]
    Comma,

    #[token(";")]
    Semicolon,

    #[token(":")]
    Colon,

    #[token(".")]
    Dot,

    #[token("=")]
    Equal,

    #[token("+")]
    Plus,

    #[token("-")]
    Minus,

    #[token("*")]
    Star,

    #[token("/")]
    Slash,

    #[token("%")]
    Percent,

    #[token("&")]
    Amp,

    #[token("|")]
    Pipe,

    #[token("^")]
    Caret,

    #[token("~")]
    Tilde,

    #[token("!")]
    Bang,

    #[token("?")]
    Question,

    #[token("\\")]
    Backslash,

    #[regex(r"/(\r\n|\n|\r)")]
    Continuation,
}

impl Token {
    pub fn is_trivial(&self) -> bool {
        matches!(self, Token::Whitespace(_) | Token::NewLine | Token::Continuation | Token::LineComment | Token::BlockComment)
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Token::NewLine => "\n",
                Token::LineComment => "",
                Token::BlockComment => "",
                Token::Hash => "#",
                Token::HashHash => "##",
                Token::Namespace => "namespace",
                Token::Inline => "inline",
                Token::Using => "using",
                Token::Typedef => "typedef",
                Token::Class => "class",
                Token::Struct => "struct",
                Token::Enum => "enum",
                Token::Union => "union",
                Token::Template => "template",
                Token::Typename => "typename",
                Token::Concept => "concept",
                Token::Requires => "requires",
                Token::Friend => "friend",
                Token::Extern => "extern",
                Token::Static => "static",
                Token::Explicit => "explicit",
                Token::Const => "const",
                Token::Mutable => "mutable",
                Token::Volatile => "volatile",
                Token::Constexpr => "constexpr",
                Token::Consteval => "consteval",
                Token::Constinit => "constinit",
                Token::Auto => "auto",
                Token::Operator => "operator",
                Token::Public => "public",
                Token::Protected => "protected",
                Token::Private => "private",
                Token::Whitespace(str)
                | Token::StringLiteral(str)
                | Token::CharacterLiteral(str)
                | Token::NumberLiteral(str)
                | Token::Identifier(str) => str,
                Token::DoubleColon => "::",
                Token::Ellipsis => "...",
                Token::Arrow => "->",
                Token::ArrowStar => "->*",
                Token::DotStar => ".*",
                Token::Spaceship => "<=>",
                Token::EqualEqual => "==",
                Token::NotEqual => "!=",
                Token::LessEqual => "<=",
                Token::GreaterEqual => ">=",
                Token::AndAnd => "&&",
                Token::OrOr => "||",
                Token::PlusPlus => "++",
                Token::MinusMinus => "--",
                Token::PlusEqual => "+=",
                Token::MinusEqual => "-=",
                Token::StarEqual => "*=",
                Token::SlashEqual => "/=",
                Token::PercentEqual => "%=",
                Token::AmpEqual => "&=",
                Token::PipeEqual => "|=",
                Token::CaretEqual => "^=",
                Token::ShiftLeft => "<<",
                Token::ShiftRight => ">>",
                Token::ShiftLeftEqual => "<<=",
                Token::ShiftRightEqual => ">>=",
                Token::LParen => "(",
                Token::RParen => ")",
                Token::LBrace => "{",
                Token::RBrace => "}",
                Token::LBracket => "[",
                Token::RBracket => "]",
                Token::Less => "<",
                Token::Greater => ">",
                Token::Comma => ",",
                Token::Semicolon => ";",
                Token::Colon => ":",
                Token::Dot => ".",
                Token::Equal => "=",
                Token::Plus => "+",
                Token::Minus => "-",
                Token::Star => "*",
                Token::Slash => "/",
                Token::Percent => "%",
                Token::Amp => "&",
                Token::Pipe => "|",
                Token::Caret => "^",
                Token::Tilde => "~",
                Token::Bang => "!",
                Token::Question => "?",
                Token::Backslash => "/",
                Token::Continuation => "/\\n",
            }
        )
    }
}
