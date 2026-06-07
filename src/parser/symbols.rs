use std::fmt;
use std::io::Write;
use chumsky::error::Rich;
use chumsky::{extra, Parser};
use chumsky::input::ValueInput;
use chumsky::prelude::{IterParser, SimpleSpan};
use chumsky::primitive::{any, choice};
use chumsky::recursive::recursive;
use crate::parser::grammar::{GuardedToken, PreprocessorGuard, Token};
use crate::writer::IndentedWriter;

#[derive(Debug)]
pub struct SymbolError<'tok> {
    pub errors: Vec<Rich<'tok, GuardedToken<'tok>>>
}

impl<'tok> fmt::Display for SymbolError<'tok> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Multiple errors occurred (count: {}):", self.errors.len())?;
        for err in &self.errors {
            writeln!(f, "  * {err}")?;
        }
        Ok(())
    }
}

impl<'tok> std::error::Error for SymbolError<'tok> {}

#[derive(Debug, Clone)]
pub struct Namespace {
    pub name: String,
    pub is_inline: bool,
    pub symbols: Vec<Symbol>,
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Namespace(Namespace)
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

fn identifier<'tok, I>(
) -> impl Parser<'tok, I, String, extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>,
{
    any().filter_map(|guarded: GuardedToken<'tok>| {
        match guarded.token {
            Token::Identifier(name) => Some(name.clone()),
            _ => None,
        }
    })
}

fn attribute<'tok, I>(
) -> impl Parser<'tok, I, String, extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>
{
    token(Token::LBracket)
        .repeated()
        .exactly(2)
        .ignore_then(identifier())
        .then_ignore(token(Token::RBracket)
            .repeated()
            .exactly(2))
}

fn attribute_list<'tok, I>(
) -> impl Parser<'tok, I, Vec<String>, extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>
{
    attribute().repeated().collect()
}

fn unknown_syntax<'tok, I>() -> impl Parser<'tok, I, (), extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>
{
   recursive(|unknown| {
       let balanced_braces = token(Token::LBrace)
           .ignore_then(unknown.repeated())
           .ignore_then(token(Token::RBrace))
           .ignored();

       let single_token = any()
           .filter(|guarded: &GuardedToken<'tok>| {
               !matches!(
                    guarded.token,
                    Token::LBrace | Token::RBrace
                )
           })
           .ignored();

       choice((
           balanced_braces,
           single_token,
       ))
   })
}

fn symbol_definition<'tok, I>(
) -> impl Parser<'tok, I, Option<Symbol>, extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>
{
    recursive(|symbol| {
        let namespace_definition = token(Token::Inline)
            .or_not()
            .map(|inline| inline.is_some())
            .then(token(Token::Namespace))
            .then(identifier())
            .then_ignore(attribute_list().or_not())
            .then_ignore(token(Token::LBrace))
            .then(symbol
                .repeated()
                .collect::<Vec<_>>()
                .map(|items| items.into_iter().filter_map(|item| item)
                    .collect()))
            .then_ignore(token(Token::RBrace))
            .map(|(((is_inline, t), name), symbols)| Some(Symbol {
                guards: t.guards.to_vec(),
                kind: SymbolKind::Namespace(Namespace { name, is_inline, symbols }),
            }));

        let unknown_symbol = unknown_syntax().to(None);

        choice((
            namespace_definition,
            unknown_symbol,
        ))
    })
}

pub fn parse_symbols<'tok, I>(
    input: I,
) -> Result<Vec<Symbol>, SymbolError<'tok>>
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>
{
    symbol_definition()
        .repeated()
        .collect::<Vec<_>>()
        .map(|symbols: Vec<Option<Symbol>>| symbols.into_iter().filter_map(|symbol| symbol).collect())
        .parse(input).into_result()
        .map_err(|err| SymbolError { errors: err })
}