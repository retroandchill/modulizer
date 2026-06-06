use std::fmt;
use std::fmt::Write;
use std::path::PathBuf;
use chumsky::IterParser;
use crate::parser::grammar::Token;
use chumsky::error::Rich;
use chumsky::input::ValueInput;
use chumsky::primitive::{any, choice, end, just};
use chumsky::span::SimpleSpan;
use chumsky::{Parser, extra, select};
use logos::Lexer;

#[derive(Debug)]
pub struct PreprocessorError<'tok> {
    pub errors: Vec<Rich<'tok, Token>>
}

impl<'tok> fmt::Display for PreprocessorError<'tok> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Multiple errors occurred (count: {}):", self.errors.len())?;
        for err in &self.errors {
            writeln!(f, "  * {err}")?;
        }
        Ok(())
    }
}

impl<'tok> std::error::Error for PreprocessorError<'tok> {}

#[derive(Debug, Clone)]
pub struct IncludeDirective {
    pub tokens: Vec<Token>,
    pub path: IncludePath
}

#[derive(Debug, Clone)]
pub enum IncludePath {
    System(PathBuf),
    Local(PathBuf),
    Macro,
}

#[derive(Debug, Clone)]
pub struct DefineDirective {
    pub name: String,
    pub parameters: Option<MacroParameters>,
    pub replacement: Vec<Token>,
}

#[derive(Debug, Clone)]
pub struct MacroParameters {
    pub names: Vec<String>,
    pub variadic: bool,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone)]
pub struct UndefDirective {
    pub name: String,
}

#[derive(Debug, Clone)]
pub enum ConditionalDirective {
    If {
        expression: Vec<Token>,
    },
    Ifdef {
        name: String,
    },
    Ifndef {
        name: String,
    },
    Elif {
        expression: Vec<Token>,
    },
    Elifdef {
        name: String,
    },
    Elifndef {
        name: String,
    },
}

#[derive(Debug, Clone)]
pub struct  OtherDirective {
    pub name: String,
    pub expression: Vec<Token>,
}

#[derive(Debug, Clone)]
pub enum DirectiveStatement {
    Include(IncludeDirective),
    Define(DefineDirective),
    Undef(UndefDirective),
    Conditional(ConditionalDirective),
    Else,
    Endif,
    Other(OtherDirective),
}

#[derive(Debug, Clone)]
pub struct CodeLine {
    pub tokens: Vec<Token>,
}


#[derive(Debug, Clone)]
pub enum PreprocessorStatement {
    Directive(DirectiveStatement),
    Code(CodeLine),
}

fn whitespace<'tok, I>() -> impl Parser<'tok, I, (), extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
{
    any()
        .filter(|token: &Token| token.is_trivial() && !matches!(token, Token::NewLine))
        .ignored()
        .repeated()
        .ignored()
}

fn newline<'tok, I>(
) -> impl Parser<'tok, I, (), extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::NewLine).ignored()
}

fn optional_newline<'tok, I>(
) -> impl Parser<'tok, I, (), extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    newline().or(end()).ignored()
}

fn rest_of_line<'tok, I>(
) -> impl Parser<'tok, I, &'tok [Token], extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    any()
        .filter(|token| !matches!(token, Token::NewLine))
        .repeated()
        .to_slice()
}

fn non_empty_rest_of_line<'tok, I>(
) -> impl Parser<'tok, I, &'tok [Token], extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    any()
        .filter(|token| !matches!(token, Token::NewLine))
        .repeated()
        .at_least(1)
        .to_slice()
}

fn directive_head<'tok, I>(
    name: &'static str,
) -> impl Parser<'tok, I, (), extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    whitespace()
        .ignore_then(just(Token::Hash))
        .ignore_then(whitespace())
        .ignore_then(just(Token::Identifier(name.to_string())))
        .ignored()
}

fn identifier<'tok, I>(
) -> impl Parser<'tok, I, String, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    select! {
        Token::Identifier(name) => name,
    }
}

fn angle_include<'tok, I>(
) -> impl Parser<'tok, I, IncludeDirective, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    just(Token::Less)
        .ignore_then(any().filter(|token| !matches!(token, Token::NewLine | Token::Greater))
            .repeated()
            .collect())
        .then_ignore(just(Token::Greater))
        .then_ignore(whitespace().or_not())
        .then_ignore(optional_newline())
        .map(|tokens| {
            let mut path = String::new();
            for token in &tokens {
                path.write_fmt(format_args!("{}", token)).unwrap();
            }

            IncludeDirective { tokens, path: IncludePath::System(PathBuf::from(path)) }
        })
}

fn quote_include<'tok, I>(
) -> impl Parser<'tok, I, IncludeDirective, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    any()
        .filter(|token| matches!(token, Token::StringLiteral(_)))
        .then_ignore(whitespace().or_not())
        .then_ignore(optional_newline())
        .map(|token: Token| {
            let Token::StringLiteral(path) = &token else {
                panic!("Expected string literal");
            };
            
            let quote_span = path.trim_matches('"');
            let escaped = deescape_string(quote_span);
            IncludeDirective { tokens: vec![token], path: IncludePath::Local(PathBuf::from(escaped)) }
        })
}

fn deescape_string(input: &str) -> String {
    let mut result = String::new();
    let mut chars = input.chars();
    
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => result.push('\n'),
                Some('r') => result.push('\r'),
                Some('t') => result.push('\t'),
                Some('\"') => result.push('"'),
                Some('\\') => result.push('\\'),
                Some(other) => {
                    result.push('\\');
                    result.push(other);
                },
                None => break
            }
        }
        else {
            result.push(c);
        }
    }
    
    result
}

fn include_directive<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    directive_head("include")
        .ignore_then(whitespace())
        .ignore_then(choice((angle_include(), quote_include())))
        .map(DirectiveStatement::Include)

}

fn macro_parameter_name<'tok, I>(
) -> impl Parser<'tok, I, Option<String>, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    choice((
        just(Token::Ellipsis).to(None),
        identifier().map(Some),
    ))
}

fn macro_parameters<'tok, I>(
) -> impl Parser<'tok, I, MacroParameters, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    just(Token::LParen)
        .ignore_then(
            macro_parameter_name()
                .separated_by(just(Token::Comma).padded_by(whitespace()))
                .allow_trailing()
                .collect::<Vec<_>>(),
        )
        .then_ignore(just(Token::RParen))
        .to_slice()
        .map(|tokens: &[Token]| {
            let mut names = Vec::new();
            let mut variadic = false;

            for token in tokens {
                match token {
                    Token::Identifier(name) => names.push(name.clone()),
                    Token::Ellipsis => variadic = true,
                    _ => {}
                }
            }

            MacroParameters {
                names,
                variadic,
                tokens: tokens.to_vec(),
            }
        })
}

fn define_directive<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    directive_head("define")
        .ignore_then(whitespace())
        .ignore_then(identifier())
        .then(macro_parameters().or_not())
        .then_ignore(whitespace())
        .then(rest_of_line())
        .then_ignore(optional_newline())
        .map(|((name, parameters), replacement)| {
            DirectiveStatement::Define(DefineDirective {
                name,
                parameters,
                replacement: replacement.to_vec(),
            })
        })
}

fn undef_directive<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    directive_head("undef")
        .ignore_then(whitespace())
        .ignore_then(identifier())
        .then_ignore(rest_of_line())
        .then_ignore(optional_newline())
        .map(|name| DirectiveStatement::Undef(UndefDirective { name }))
}

fn if_directive<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    directive_head("if")
        .ignore_then(whitespace())
        .ignore_then(rest_of_line())
        .then_ignore(optional_newline())
        .map(|expression| {
            DirectiveStatement::Conditional(ConditionalDirective::If { expression: expression.to_vec() })
        })
}

fn ifdef_directive<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    directive_head("ifdef")
        .ignore_then(whitespace())
        .ignore_then(identifier())
        .then_ignore(rest_of_line())
        .then_ignore(optional_newline())
        .map(|name| DirectiveStatement::Conditional(ConditionalDirective::Ifdef { name }))
}

fn ifndef_directive<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    directive_head("ifndef")
        .ignore_then(whitespace())
        .ignore_then(identifier())
        .then_ignore(rest_of_line())
        .then_ignore(optional_newline())
        .map(|name| DirectiveStatement::Conditional(ConditionalDirective::Ifndef { name }))
}

fn elif_directive<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    directive_head("elif")
        .ignore_then(whitespace())
        .ignore_then(rest_of_line())
        .then_ignore(optional_newline())
        .map(|expression| {
            DirectiveStatement::Conditional(ConditionalDirective::Elif { expression: expression.to_vec() })
        })
}

fn elifdef_directive<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    directive_head("elifdef")
        .ignore_then(whitespace())
        .ignore_then(identifier())
        .then_ignore(rest_of_line())
        .then_ignore(optional_newline())
        .map(|name| DirectiveStatement::Conditional(ConditionalDirective::Elifdef { name }))
}

fn elifndef_directive<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    directive_head("elifndef")
        .ignore_then(whitespace())
        .ignore_then(identifier())
        .then_ignore(rest_of_line())
        .then_ignore(optional_newline())
        .map(|name| DirectiveStatement::Conditional(ConditionalDirective::Elifndef { name }))
}

fn else_directive<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    directive_head("else")
        .then_ignore(rest_of_line())
        .then_ignore(optional_newline())
        .to(DirectiveStatement::Else)
}

fn endif_directive<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    directive_head("endif")
        .then_ignore(rest_of_line())
        .then_ignore(optional_newline())
        .to(DirectiveStatement::Endif)
}

fn other_directive<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    whitespace()
        .ignore_then(just(Token::Hash))
        .ignore_then(whitespace())
        .ignore_then(identifier())
        .then_ignore(whitespace())
        .then(rest_of_line())
        .then_ignore(optional_newline())
        .map(|(name, expression)| {
            DirectiveStatement::Other(OtherDirective { name, expression: expression.to_vec() })
        })
}

fn directive_statement<'tok, I>(
) -> impl Parser<'tok, I, DirectiveStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    choice((
        include_directive(),
        define_directive(),
        undef_directive(),
        ifdef_directive(),
        ifndef_directive(),
        if_directive(),
        elifdef_directive(),
        elifndef_directive(),
        elif_directive(),
        else_directive(),
        endif_directive(),
        other_directive(),
    ))
}

fn blank_line<'tok, I>(
) -> impl Parser<'tok, I, PreprocessorStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    newline().to(PreprocessorStatement::Code(CodeLine { tokens: Vec::new() }))
}

fn code_line<'tok, I>(
) -> impl Parser<'tok, I, CodeLine, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    non_empty_rest_of_line()
        .then_ignore(optional_newline())
        .map(|tokens| CodeLine { tokens: tokens.to_vec() })
}

fn statement<'tok, I>(
) -> impl Parser<'tok, I, PreprocessorStatement, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    choice((
        directive_statement().map(PreprocessorStatement::Directive),
        blank_line(),
        code_line().map(PreprocessorStatement::Code),
        ))
}

fn statements<'tok, I>(
) -> impl Parser<
    'tok,
    I,
    Vec<PreprocessorStatement>,
    extra::Err<Rich<'tok, Token>>,
>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
    + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    statement().repeated().collect()
}

pub fn collect_statements(
    tokens: &[Token],
) -> Result<Vec<PreprocessorStatement>, PreprocessorError<'_>> {
    statements().parse(tokens).into_result()
        .map_err(|errs| PreprocessorError {
            errors: errs,
        })
}