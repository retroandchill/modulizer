use crate::parser::grammar::preprocessor::ConditionalDirective;
use logos::{Lexer, Logos};
use std::fmt;
use std::fmt::Formatter;
use std::rc::Rc;
use ustr::Ustr;

#[derive(Logos, Clone, Debug, PartialEq)]
pub enum Token {
    #[regex(r"\r\n|\n|\r")]
    NewLine,

    #[regex(r"[ \t\f]+")]
    Whitespace,

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

    #[token("virtual")]
    Virtual,

    #[token("friend")]
    Friend,

    #[token("extern")]
    Extern,

    #[token("static")]
    Static,

    #[token("thread_local")]
    ThreadLocal,

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

    #[token("decltype")]
    Decltype,

    #[token("operator")]
    Operator,

    #[token("noexcept")]
    Noexcept,

    #[token("final")]
    Final,

    #[token("public")]
    Public,

    #[token("protected")]
    Protected,

    #[token("private")]
    Private,

    #[token("void")]
    Void,

    #[token("bool")]
    Bool,

    #[token("int")]
    Int,

    #[token("signed")]
    Signed,

    #[token("unsigned")]
    Unsigned,

    #[token("short")]
    Short,

    #[token("long")]
    Long,

    #[token("long long")]
    LongLong,

    #[token("float")]
    Float,

    #[token("double")]
    Double,

    #[token("char")]
    Char,

    #[token("wchar_t")]
    WChar,

    #[token("char8_t")]
    Char8,

    #[token("char16_t")]
    Char16,

    #[token("char32_t")]
    Char32,

    #[token("new")]
    New,

    #[token("delete")]
    Delete,

    #[token("co_await")]
    CoAwait,

    #[regex(r#"(u8|u|U|L)?""#, parse_string_literal)]
    StringLiteral(Ustr),

    #[regex(r"(u8|u|U|L)?'([^'\\]|\\.|\\\r?\n)*'", |lex| Ustr::from(lex.slice()))]
    CharacterLiteral(Ustr),

    #[regex(r"([0-9][0-9']*)(\.[0-9']*)?([eEpP][+-]?[0-9']+)?[a-zA-Z_0-9]*", |lex| Ustr::from(lex.slice()))]
    NumberLiteral(Ustr),

    // Identifiers.
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", |lex| Ustr::from(lex.slice()))]
    Identifier(Ustr),

    // Multi-character operators and punctuation.
    #[token("::")]
    DoubleColon,

    #[token("...")]
    Ellipsis,

    #[token("->")]
    Arrow,

    #[token("==")]
    EqualEqual,

    #[token("&&")]
    And,

    #[token("||")]
    Or,

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

    #[regex(r"\\(\r\n|\n|\r)")]
    Continuation,
}

impl Token {
    pub fn is_trivial(&self) -> bool {
        matches!(
            self,
            Token::Whitespace
                | Token::NewLine
                | Token::Continuation
                | Token::LineComment
                | Token::BlockComment
        )
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
                Token::Virtual => "virtual",
                Token::Extern => "extern",
                Token::Static => "static",
                Token::ThreadLocal => "thread_local",
                Token::Explicit => "explicit",
                Token::Const => "const",
                Token::Mutable => "mutable",
                Token::Volatile => "volatile",
                Token::Constexpr => "constexpr",
                Token::Consteval => "consteval",
                Token::Constinit => "constinit",
                Token::Decltype => "decltype",
                Token::Auto => "auto",
                Token::Operator => "operator",
                Token::Public => "public",
                Token::Protected => "protected",
                Token::Private => "private",
                Token::Noexcept => "noexcept",
                Token::Final => "final",
                Token::Whitespace => " ",
                Token::StringLiteral(str)
                | Token::CharacterLiteral(str)
                | Token::NumberLiteral(str)
                | Token::Identifier(str) => str,
                Token::DoubleColon => "::",
                Token::Ellipsis => "...",
                Token::Arrow => "->",
                Token::EqualEqual => "==",
                Token::And => "&&",
                Token::Or => "||",
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
                Token::Continuation => "\\\n",
                Token::Void => "void",
                Token::Bool => "bool",
                Token::Int => "int",
                Token::Signed => "signed",
                Token::Unsigned => "unsigned",
                Token::Short => "short",
                Token::Long => "long",
                Token::LongLong => "long long",
                Token::Float => "float",
                Token::Double => "double",
                Token::Char => "char",
                Token::WChar => "wchar_t",
                Token::Char8 => "char8_t",
                Token::Char16 => "char16_t",
                Token::Char32 => "char32_t",
                Token::New => "new",
                Token::Delete => "delete",
                Token::CoAwait => "co_await",
            }
        )
    }
}

fn parse_string_literal(lex: &mut Lexer<Token>) -> Option<Ustr> {
    let remainder = lex.remainder();
    let bytes = remainder.as_bytes();

    let mut escaped = false;
    let mut end = None;

    for (index, byte) in bytes.iter().enumerate() {
        match (*byte, escaped) {
            (b'\\', false) => escaped = true,
            (b'"', false) => {
                end = Some(index + 1);
                break;
            }
            _ => escaped = false,
        }
    }

    let end = end?;
    lex.bump(end);

    Some(Ustr::from(lex.slice()))
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreprocessorGuard {
    Conditional(ConditionalDirective),
    Else,
}

#[derive(Debug, Clone)]
pub struct GuardedTokens {
    guards: Rc<[PreprocessorGuard]>,
    tokens: Vec<Token>,
}

impl GuardedTokens {
    pub fn new(guards: impl Iterator<Item = PreprocessorGuard>) -> Self {
        Self {
            guards: guards.collect(),
            tokens: Vec::new(),
        }
    }

    pub fn append(&mut self, tokens: impl Iterator<Item = Token>) {
        self.tokens
            .extend(tokens.filter(|token| !token.is_trivial()));
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GuardedToken {
    pub guards: Rc<[PreprocessorGuard]>,
    pub token: Token,
}

impl fmt::Display for GuardedToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.token)
    }
}

pub struct GuardedTokenIterator<'a> {
    tokens: &'a GuardedTokens,
    index: Option<usize>,
}

impl<'a> Iterator for GuardedTokenIterator<'a> {
    type Item = GuardedToken;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(index) = self.index {
            self.index = Some(index + 1);
        } else {
            self.index = Some(0);
        }

        let index = self.index.unwrap();
        self.tokens.tokens.get(index).map(|token| GuardedToken {
            guards: self.tokens.guards.clone(),
            token: token.clone(),
        })
    }
}

impl<'a> IntoIterator for &'a GuardedTokens {
    type Item = GuardedToken;
    type IntoIter = GuardedTokenIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            tokens: self,
            index: None,
        }
    }
}
