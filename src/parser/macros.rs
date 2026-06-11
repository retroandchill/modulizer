use crate::parser::common::{identifier, whitespace, PreprocessorError};
use crate::parser::grammar::Token;
use chumsky::error::Rich;
use chumsky::input::ValueInput;
use chumsky::prelude::SimpleSpan;
use chumsky::primitive::{any, choice, just};
use chumsky::recursive::recursive;
use chumsky::{extra, IterParser, Parser};
use ustr::Ustr;

#[derive(Debug, Clone)]
pub struct MacroExpansionCandidate {
    pub name: Ustr,
    pub parameters: Option<Vec<Vec<Token>>>
}

#[derive(Debug, Clone)]
pub enum ExpandableSyntax {
    Candidate(MacroExpansionCandidate),
    Expression(Vec<Token>),
}

fn macro_parameters<'tok, I>() -> impl Parser<'tok, I, Vec<Vec<Token>>, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    let nested = recursive(|nested| {
        choice((
            just(Token::LParen)
                .ignore_then(nested.clone())
                .then_ignore(just(Token::RParen))
                .map(|tokens: Vec<Token>| {
                    let mut grouped = Vec::with_capacity(tokens.len() + 2);
                    grouped.push(Token::LParen);
                    grouped.extend(tokens);
                    grouped.push(Token::RParen);
                    grouped
                }),
            any()
                .filter(|token| !matches!(token, Token::LParen | Token::RParen))
                .map(|token| vec![token]),
        ))
            .repeated()
            .collect::<Vec<_>>()
            .map(|chunks| chunks.into_iter().flatten().collect::<Vec<_>>())
    });

    let argument = choice((
        just(Token::LParen)
            .ignore_then(nested)
            .then_ignore(just(Token::RParen))
            .map(|tokens: Vec<Token>| {
                let mut grouped = Vec::with_capacity(tokens.len() + 2);
                grouped.push(Token::LParen);
                grouped.extend(tokens);
                grouped.push(Token::RParen);
                grouped
            }),
        any()
            .filter(|token| {
                !matches!(
                    token,
                    Token::LParen | Token::RParen | Token::Comma
                )
            })
            .map(|token| vec![token]),
    ))
        .repeated()
        .collect::<Vec<_>>()
        .map(|chunks| chunks.into_iter().flatten().collect::<Vec<_>>());

    whitespace()
        .or_not()
        .ignore_then(
            just(Token::LParen)
                .ignore_then(
                    argument
                        .separated_by(just(Token::Comma))
                        .collect::<Vec<_>>(),
                )
                .then_ignore(just(Token::RParen)),
        )
}

fn candidate<'tok, I>() -> impl Parser<'tok, I, MacroExpansionCandidate, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan>,
{
    identifier()
        .then(whitespace().or_not().ignore_then(macro_parameters()).or_not())
        .map(|(name, parameters)| MacroExpansionCandidate { name, parameters })
}

fn expandable_syntax<'tok, I>() -> impl Parser<'tok, I, ExpandableSyntax, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan> + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>
{
    candidate().map(ExpandableSyntax::Candidate)
        .or(any()
            .filter(|token| !matches!(token, Token::Identifier(_)))
            .repeated()
            .at_least(1)
            .collect()
            .map(|tokens| ExpandableSyntax::Expression(tokens)))
}

fn all_syntax<'tok, I>() -> impl Parser<'tok, I, Vec<ExpandableSyntax>, extra::Err<Rich<'tok, Token>>>
where
    I: ValueInput<'tok, Token = Token, Span = SimpleSpan> + chumsky::input::SliceInput<'tok, Slice = &'tok [Token]>
{
    expandable_syntax().repeated().collect()
}

pub fn parse_expandable_syntax(source: &[Token]) -> Result<Vec<ExpandableSyntax>, PreprocessorError<'_>> {
    all_syntax().parse(source).into_result().map_err(|errs| PreprocessorError {
        errors: errs,
    })
}