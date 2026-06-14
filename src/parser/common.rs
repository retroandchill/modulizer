use crate::parser::grammar::Token;
use chumsky::error::Rich;
use chumsky::input::ValueInput;
use chumsky::prelude::{SimpleSpan, any, end, just};
use chumsky::{Parser, extra, select};
use std::fmt;
use ustr::Ustr;

#[derive(Debug)]
pub struct PreprocessorError<'tok> {
    pub errors: Vec<Rich<'tok, Token>>,
}

impl<'tok> fmt::Display for PreprocessorError<'tok> {
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

impl<'tok> std::error::Error for PreprocessorError<'tok> {}

pub fn whitespace<'tok, I>() -> impl Parser<'tok, I, (), extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    any()
        .filter(|token: &Token| token.is_trivial())
        .ignored()
        .repeated()
        .ignored()
}

pub fn non_breaking_whitespace<'tok, I>() -> impl Parser<'tok, I, (), extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    any()
        .filter(|token: &Token| token.is_trivial() && !matches!(token, Token::NewLine))
        .ignored()
        .repeated()
        .ignored()
}

pub fn newline<'tok, I>() -> impl Parser<'tok, I, (), extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    just(Token::NewLine).ignored()
}

pub fn optional_newline<'tok, I>() -> impl Parser<'tok, I, (), extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    newline().or(end()).ignored()
}

pub fn rest_of_line<'tok, I>() -> impl Parser<'tok, I, &'tok [Token], extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>
        + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>,
{
    any()
        .filter(|token| !matches!(token, Token::NewLine))
        .repeated()
        .to_slice()
}

pub fn non_empty_rest_of_line<'tok, I>()
-> impl Parser<'tok, I, &'tok [Token], extra::Err<Rich<'tok, Token>>>
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

pub fn identifier<'tok, I>() -> impl Parser<'tok, I, Ustr, extra::Err<Rich<'tok, Token>>> + Clone
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    select! {
        Token::Identifier(name) => name,
    }
}
