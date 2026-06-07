use crate::parser::grammar::{GuardedToken, PreprocessorGuard, Token};
use chumsky::error::Rich;
use chumsky::input::{SliceInput, ValueInput};
use chumsky::prelude::{IterParser, SimpleSpan};
use chumsky::primitive::{any, choice};
use chumsky::recursive::recursive;
use chumsky::{Parser, extra};
use std::fmt;
use std::fmt::Write;

#[derive(Debug)]
pub struct SymbolError<'tok> {
    pub errors: Vec<Rich<'tok, GuardedToken<'tok>>>,
}

impl<'tok> fmt::Display for SymbolError<'tok> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "Multiple errors occurred (count: {}):",
            self.errors.len()
        )?;
        for err in &self.errors {
            writeln!(f, "  * {err}")?;
        }
        Ok(())
    }
}

impl<'tok> std::error::Error for SymbolError<'tok> {}

#[derive(Debug, Clone)]
pub struct CppNameSegment {
    pub name: String,
    pub has_template_args: bool
}

#[derive(Debug, Clone)]
pub struct Namespace {
    pub name: String,
    pub is_inline: bool,
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Namespace(Namespace),
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub guards: Vec<PreprocessorGuard>,
    pub kind: SymbolKind,
}

fn token<'tok, I>(
    expected: Token,
) -> impl Parser<'tok, I, GuardedToken<'tok>, extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>,
{
    any().filter(move |guarded: &GuardedToken<'tok>| *guarded.token == expected)
}

fn identifier<'tok, I>()
-> impl Parser<'tok, I, String, extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>,
{
    any().filter_map(|guarded: GuardedToken<'tok>| match guarded.token {
        Token::Identifier(name) => Some(name.clone()),
        _ => None,
    })
}

fn scoped_identifier<'tok, I>()
-> impl Parser<'tok, I, String, extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>,
{
    identifier()
        .separated_by(token(Token::DoubleColon))
        .collect::<Vec<_>>()
        .map(|parts: Vec<String>| parts.join("::"))
}

fn template_argument_list<'tok, I>()
-> impl Parser<'tok, I, String, extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>
        + SliceInput<'tok, Slice = &'tok [GuardedToken<'tok>]>,
{
    recursive(|template_argument| {
        token(Token::Less)
            .ignore_then(
                choice((
                    template_argument,
                    any()
                        .filter(|guarded: &GuardedToken<'tok>| {
                            !matches!(guarded.token, Token::Less | Token::Greater)
                        })
                        .ignored(),
                ))
                .repeated(),
            )
            .then_ignore(token(Token::Greater))
            .ignored()
    })
    .to_slice()
    .map(|parts: &'tok [GuardedToken<'tok>]| {
        let mut result = String::new();
        for part in parts {
            result.write_fmt(format_args!("{}", part.token)).unwrap();
        }
        result
    })
}

fn templatable_segment<'tok, I>()
    -> impl Parser<'tok, I, CppNameSegment, extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>
        + SliceInput<'tok, Slice = &'tok [GuardedToken<'tok>]>,
{
    identifier()
        .then(
            template_argument_list()
                .or_not(),
        )
        .map(|(name, template_arguments)| {
            match template_arguments {
                Some(a) => CppNameSegment { name: format!("{}{}", name, a), has_template_args: true },
                None => CppNameSegment { name, has_template_args: false },
            }
        })
}

fn attribute<'tok, I>()
    -> impl Parser<'tok, I, String, extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>,
{
    token(Token::LBracket)
        .repeated()
        .exactly(2)
        .ignore_then(scoped_identifier())
        .then_ignore(token(Token::RBracket).repeated().exactly(2))
}

fn attribute_list<'tok, I>()
    -> impl Parser<'tok, I, Vec<String>, extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>,
{
    attribute().repeated().collect()
}

fn unknown_syntax<'tok, I>()
-> impl Parser<'tok, I, (), extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>,
{
    recursive(|unknown| {
        let balanced_braces = token(Token::LBrace)
            .ignore_then(unknown.repeated())
            .ignore_then(token(Token::RBrace))
            .ignored();

        let single_token = any()
            .filter(|guarded: &GuardedToken<'tok>| {
                !matches!(guarded.token, Token::LBrace | Token::RBrace)
            })
            .ignored();

        choice((balanced_braces, single_token))
    })
}

fn symbol_definition<'tok, I>()
-> impl Parser<'tok, I, Option<Symbol>, extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>,
{
    recursive(|symbol| {
        let namespace_definition = token(Token::Inline)
            .or_not()
            .map(|inline| inline.is_some())
            .then(token(Token::Namespace))
            .then(scoped_identifier())
            .then_ignore(attribute_list().or_not())
            .then_ignore(token(Token::LBrace))
            .then(
                symbol
                    .repeated()
                    .collect::<Vec<_>>()
                    .map(|items| items.into_iter().filter_map(|item| item).collect()),
            )
            .then_ignore(token(Token::RBrace))
            .map(|(((is_inline, t), name), symbols)| {
                Some(Symbol {
                    guards: t.guards.to_vec(),
                    kind: SymbolKind::Namespace(Namespace {
                        name,
                        is_inline,
                        symbols,
                    }),
                })
            });

        let unknown_symbol = unknown_syntax().to(None);

        choice((namespace_definition, unknown_symbol))
    })
}

pub fn parse_symbols<'tok, I>(input: I) -> Result<Vec<Symbol>, SymbolError<'tok>>
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>,
{
    symbol_definition()
        .repeated()
        .collect::<Vec<_>>()
        .map(|symbols: Vec<Option<Symbol>>| {
            symbols.into_iter().filter_map(|symbol| symbol).collect()
        })
        .parse(input)
        .into_result()
        .map_err(|err| SymbolError { errors: err })
}
