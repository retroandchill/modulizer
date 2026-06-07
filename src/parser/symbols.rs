use crate::parser::grammar::{GuardedToken, PreprocessorGuard, Token};
use chumsky::error::Rich;
use chumsky::input::{SliceInput, ValueInput};
use chumsky::prelude::{IterParser, SimpleSpan};
use chumsky::primitive::{any, choice};
use chumsky::recursive::recursive;
use chumsky::{Parser, extra};
use std::fmt;

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
    Class {
        name: String,
    },
    Struct {
        name: String,
    },
    Union {
        name: String,
    },
    Enum {
        name: String,
        scoped: bool,
    },
    Using {
        name: String,
        namespace: bool
    },
    Typedef {
        name: String,
    },
    Function {
        name: String,
    },
    Variable {
        name: String,
    },
    Concept {
        name: String,
    }
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

fn balanced_group<'tok, I>(
    open: Token,
    close: Token,
) -> impl Parser<'tok, I, (), extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>,
{
    recursive(move |group| {
        token(open.clone())
            .ignore_then(
                choice((
                    group,
                    any()
                        .filter({
                            let open = open.clone();
                            let close = close.clone();
                            move |guarded: &GuardedToken<'tok>| {
                                *guarded.token != open && *guarded.token != close
                            }
                        })
                        .ignored(),
                ))
                    .repeated(),
            )
            .then_ignore(token(close.clone()))
            .ignored()
    })
}

fn declaration_chunk<'tok, I>()
    -> impl Parser<'tok, I, &'tok [GuardedToken<'tok>], extra::Err<Rich<'tok, GuardedToken<'tok>>>> + Clone
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>
    + SliceInput<'tok, Slice = &'tok [GuardedToken<'tok>]>,
{
    choice((
        balanced_group(Token::LParen, Token::RParen),
        balanced_group(Token::LBracket, Token::RBracket),
        balanced_group(Token::Less, Token::Greater),
        balanced_group(Token::LBrace, Token::RBrace),
        any()
            .filter(|guarded: &GuardedToken<'tok>| {
                !matches!(
                    guarded.token,
                    Token::Semicolon
                        | Token::LParen
                        | Token::RParen
                        | Token::LBracket
                        | Token::RBracket
                        | Token::Less
                        | Token::Greater
                        | Token::LBrace
                        | Token::RBrace
                )
            })
            .ignored(),
    ))
        .repeated()
        .at_least(1)
        .then_ignore(token(Token::Semicolon).or_not())
        .to_slice()
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
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>
    + SliceInput<'tok, Slice = &'tok [GuardedToken<'tok>]>,
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

        let declaration = declaration_chunk()
            .map(classify_declaration_chunk);

        let unknown_symbol = unknown_syntax().to(None);

        choice((namespace_definition,
                declaration,
                unknown_symbol))
    })
}

fn classify_declaration_chunk(tokens: &[GuardedToken]) -> Option<Symbol> {
    let first = tokens.first()?;
    let guards = first.guards.to_vec();

    let mut significant = tokens
        .iter()
        .filter(|guarded| {
            !matches!(
                guarded.token,
                Token::Semicolon
                    | Token::LBrace
                    | Token::RBrace
                    | Token::LParen
                    | Token::RParen
                    | Token::LBracket
                    | Token::RBracket
                    | Token::Less
                    | Token::Greater
                    | Token::Comma
                    | Token::Colon
                    | Token::Equal
                    | Token::Star
                    | Token::Amp
            )
        })
        .peekable();

    let mut saw_template = false;
    let mut saw_typedef = false;
    let mut saw_using = false;
    let mut saw_namespace = false;

    while let Some(guarded) = significant.next() {
        match guarded.token {
            Token::Template => saw_template = true,
            Token::Typedef => saw_typedef = true,
            Token::Using => saw_using = true,
            Token::Namespace => saw_namespace = true,
            Token::Class => {
                if let Some(name) = next_identifier(&mut significant) {
                    return Some(Symbol {
                        guards,
                        kind: SymbolKind::Class { name },
                    });
                }
            },
            Token::Struct => {
                if let Some(name) = next_identifier(&mut significant) {
                    return Some(Symbol {
                        guards,
                        kind: SymbolKind::Struct { name },
                    });
                }
            },
            Token::Union => {
                if let Some(name) = next_identifier(&mut significant) {
                    return Some(Symbol {
                        guards,
                        kind: SymbolKind::Union { name },
                    });
                }
            }
            Token::Concept => {
                if let Some(name) = next_identifier(&mut significant) {
                    return Some(Symbol {
                        guards,
                        kind: SymbolKind::Concept { name },
                    });
                }
            }
            Token::Enum => {
                let mut scoped = false;

                if let Some(next) = significant.peek() {
                    scoped = matches!(next.token, Token::Class | Token::Struct);
                    if scoped {
                        significant.next();
                    }
                }

                if let Some(name) = next_identifier(&mut significant) {
                    return Some(Symbol {
                        guards,
                        kind: SymbolKind::Enum { name, scoped },
                    });
                }
            }
            Token::Identifier(name) if saw_using => {
                return Some(Symbol {
                    guards,
                    kind: SymbolKind::Using {
                        name: name.clone(),
                        namespace: saw_namespace
                    },
                });
            }
            Token::Identifier(_) if saw_typedef => {
                let name = last_identifier(tokens)?;
                return Some(Symbol {
                    guards,
                    kind: SymbolKind::Typedef { name },
                });
            }
            Token::Identifier(_) if saw_template => {
                // Explicit instantiations/declarations commonly look like:
                // template FMT_API auto foo(...) -> bar;
                // Keep this broad and use the token before the first top-level '(' as the name.
                if let Some(name) = name_before_first_paren(tokens) {
                    return Some(Symbol {
                        guards,
                        kind: SymbolKind::Function { name },
                    });
                }
            }
            _ => {},
        }
    }

    if let Some(name) = name_before_first_paren(tokens) {
        return Some(Symbol {
            guards,
            kind: SymbolKind::Function { name },
        });
    }

    if let Some(name) = variable_name(tokens) {
        return Some(Symbol {
            guards,
            kind: SymbolKind::Variable { name },
        });   
    }

    None
}

fn next_identifier<'a>(tokens: &mut impl Iterator<Item = &'a GuardedToken<'a>>) -> Option<String> {
    for guarded in tokens {
        if let Token::Identifier(name) = &guarded.token {
            return Some(name.clone());
        }
    }

    None
}

fn last_identifier(tokens: &[GuardedToken]) -> Option<String> {
    tokens.iter().rev().find_map(|guarded| {
        if let Token::Identifier(name) = &guarded.token {
            Some(name.clone())
        } else {
            None
        }
    })
}

fn name_before_first_paren(tokens: &[GuardedToken]) -> Option<String> {
    let paren_index = tokens
        .iter()
        .position(|guarded| matches!(guarded.token, Token::LParen))?;

    tokens[..paren_index].iter().rev().find_map(|guarded| {
        if let Token::Identifier(name) = &guarded.token {
            Some(name.clone())
        } else {
            None
        }
    })
}

fn variable_name(tokens: &[GuardedToken<'_>]) -> Option<String> {
    let declarator_start = first_variable_declarator_start(tokens)?;

    tokens[declarator_start..]
        .iter()
        .take_while(|guarded| {
            !matches!(
                guarded.token,
                Token::Equal
                    | Token::Comma
                    | Token::Semicolon
                    | Token::LBrace
                    | Token::RBrace
            )
        })
        .filter_map(|guarded| {
            if let Token::Identifier(name) = guarded.token {
                Some(name.clone())
            } else {
                None
            }
        })
        .last()
}

fn first_variable_declarator_start<'a>(tokens: &'a [GuardedToken<'a>]) -> Option<usize> {
    let mut saw_typeish_token = false;

    for (index, guarded) in tokens.iter().enumerate() {
        match guarded.token {
            Token::Identifier(_) => {
                if saw_typeish_token && !identifier_is_probably_macro_attribute(tokens, index) {
                    return Some(index);
                }

                saw_typeish_token = true;
            }
            Token::Auto
            | Token::Const
            | Token::Volatile
            | Token::Constexpr
            | Token::Consteval
            | Token::Constinit
            | Token::Static
            | Token::Extern
            | Token::Inline
            | Token::Mutable
            | Token::Typename
            | Token::Class
            | Token::Struct
            | Token::Enum
            | Token::Union => {
                saw_typeish_token = true;
            }
            Token::Star | Token::Amp | Token::DoubleColon | Token::Less | Token::Greater => {}
            Token::LParen | Token::RParen | Token::LBracket | Token::RBracket => {}
            _ => {}
        }
    }

    None
}

fn identifier_is_probably_macro_attribute(tokens: &[GuardedToken<'_>], index: usize) -> bool {
    let Some(Token::Identifier(name)) = tokens.get(index).map(|guarded| guarded.token) else {
        return false;
    };

    if !name.chars().all(|ch| ch.is_ascii_uppercase() || ch == '_' || ch.is_ascii_digit()) {
        return false;
    }

    tokens
        .get(index + 1)
        .is_some_and(|guarded| matches!(guarded.token, Token::Identifier(_) | Token::Auto | Token::Const))
}

pub fn parse_symbols<'tok, I>(input: I) -> Result<Vec<Symbol>, SymbolError<'tok>>
where
    I: ValueInput<'tok, Token = GuardedToken<'tok>, Span = SimpleSpan>
    + SliceInput<'tok, Slice = &'tok [GuardedToken<'tok>]>,
{
    eprintln!("parse_symbols: starting");

    let result = symbol_definition()
        .repeated()
        .collect::<Vec<_>>()
        .map(|symbols: Vec<Option<Symbol>>| {
            symbols.into_iter().filter_map(|symbol| symbol).collect()
        })
        .parse(input)
        .into_result()
        .map_err(|err| SymbolError { errors: err });

    eprintln!("parse_symbols: finished");

    result
}