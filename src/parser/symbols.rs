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

struct SymbolParser<'tok> {
    tokens: &'tok [GuardedToken<'tok>],
    index: usize,
}

impl<'tok> SymbolParser<'tok> {
    fn new(tokens: &'tok [GuardedToken<'tok>]) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse(mut self) -> Vec<Symbol> {
        self.parse_until(None)
    }

    fn parse_until(&mut self, end: Option<Token>) -> Vec<Symbol> {
        let mut symbols = Vec::new();

        while !self.is_at_end() {
            if let Some(end) = &end {
                if self.check(end) {
                    break;
                }
            }

            if self.is_at_end() {
                break;
            }

            if let Some(symbol) = self.parse_symbol() {
                symbols.push(symbol);
            } else {
                self.advance();
            }
        }

        symbols
    }

    fn parse_symbol(&mut self) -> Option<Symbol> {
        if self.check(&Token::Inline) && self.check_next(&Token::Namespace) {
            return self.parse_namespace();
        }

        if self.check(&Token::Namespace) {
            return self.parse_namespace();
        }

        let chunk = self.collect_declaration_chunk();

        if chunk.is_empty() {
            return None;
        }

        classify_declaration_chunk(chunk)
    }

    fn parse_namespace(&mut self) -> Option<Symbol> {
        let first = self.peek()?.clone();

        let is_inline = self.match_token(&Token::Inline);

        if !self.match_token(&Token::Namespace) {
            return None;
        }

        let name = self.parse_scoped_identifier()?;

        self.skip_attributes();

        if !self.match_token(&Token::LBrace) {
            return Some(Symbol {
                guards: first.guards.to_vec(),
                kind: SymbolKind::Namespace(Namespace {
                    name,
                    is_inline,
                    symbols: Vec::new(),
                }),
            })
        }

        let symbols = self.parse_until(Some(Token::RBrace));
        self.match_token(&Token::RBrace);

        Some(Symbol {
            guards: first.guards.to_vec(),
            kind: SymbolKind::Namespace(Namespace {
                name,
                is_inline,
                symbols,
            }),
        })
    }

    fn collect_declaration_chunk(&mut self) -> &'tok [GuardedToken<'tok>] {
        let start = self.index;

        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut angle_depth = 0usize;
        let mut brace_depth = 0usize;

        while let Some(guarded) = self.peek() {
            match guarded.token {
                Token::Semicolon
                if paren_depth == 0
                    && bracket_depth == 0
                    && angle_depth == 0
                    && brace_depth == 0 =>
                    {
                        self.advance();
                        break;
                    }

                Token::LBrace
                if paren_depth == 0 && bracket_depth == 0 && angle_depth == 0 =>
                    {
                        brace_depth += 1;
                        self.advance();

                        while brace_depth > 0 {
                            let Some(guarded) = self.peek() else {
                                break;
                            };

                            match guarded.token {
                                Token::LBrace => brace_depth += 1,
                                Token::RBrace => brace_depth -= 1,
                                _ => {}
                            }

                            self.advance();
                        }

                        if self.check(&Token::Semicolon) {
                            self.advance();
                        }

                        break;
                    }

                Token::RBrace
                if paren_depth == 0
                    && bracket_depth == 0
                    && angle_depth == 0
                    && brace_depth == 0 =>
                    {
                        break;
                    }

                Token::LParen => {
                    paren_depth += 1;
                    self.advance();
                }
                Token::RParen => {
                    paren_depth = paren_depth.saturating_sub(1);
                    self.advance();
                }
                Token::LBracket => {
                    bracket_depth += 1;
                    self.advance();
                }
                Token::RBracket => {
                    bracket_depth = bracket_depth.saturating_sub(1);
                    self.advance();
                }
                Token::Less => {
                    angle_depth += 1;
                    self.advance();
                }
                Token::Greater => {
                    angle_depth = angle_depth.saturating_sub(1);
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }

        &self.tokens[start..self.index]
    }

    fn parse_scoped_identifier(&mut self) -> Option<String> {
        let mut parts = Vec::new();

        let Token::Identifier(name) = self.peek()?.token else {
            return None;
        };

        parts.push(name.clone());
        self.advance();

        while self.match_token(&Token::DoubleColon) {
            let Some(guarded) = self.peek() else {
                break;
            };

            let Token::Identifier(name) = guarded.token else {
                break;
            };

            parts.push(name.clone());
            self.advance();
        }

        Some(parts.join("::"))
    }

    fn skip_attributes(&mut self) {
        loop {
            let start = self.index;

            if !self.match_token(&Token::LBracket) {
                return;
            }

            if !self.match_token(&Token::LBracket) {
                self.index = start;
                return;
            }

            let mut depth = 1usize;

            while let Some(guarded) = self.peek() {
                match guarded.token {
                    Token::LBracket => depth += 1,
                    Token::RBracket => {
                        depth = depth.saturating_sub(1);

                        if depth == 0 {
                            self.advance();

                            if self.check(&Token::RBracket) {
                                self.advance();
                            }

                            break;
                        }
                    }
                    _ => {}
                }

                self.advance();
            }
        }
    }

    fn match_token(&mut self, expected: &Token) -> bool {
        if self.check(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, expected: &Token) -> bool {
        self.peek()
            .is_some_and(|guarded| *guarded.token == *expected)
    }

    fn check_next(&self, expected: &Token) -> bool {
        self.tokens
            .get(self.index + 1)
            .is_some_and(|guarded| *guarded.token == *expected)
    }

    fn peek(&self) -> Option<&GuardedToken<'tok>> {
        self.tokens.get(self.index)
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn is_at_end(&self) -> bool {
        self.index >= self.tokens.len()
    }
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

pub fn parse_symbols<'tok>(input: &'tok [GuardedToken<'tok>]) -> Result<Vec<Symbol>, SymbolError<'tok>>
{
    eprintln!("parse_symbols: starting");

    let result = SymbolParser::new(input).parse();

    eprintln!("parse_symbols: finished");

    Ok(result)
}