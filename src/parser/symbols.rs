use crate::parser::grammar::{GuardedToken, PreprocessorGuard, Token};
use chumsky::error::Rich;
use chumsky::Parser;
use std::fmt;
use clap::builder::TypedValueParser;

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
    TypeAlias {
        name: String,
    },
    UsingNamespace {
        name: String,
    },
    UsingDeclaration {
        name: String,
    },
    NamespaceAlias {
        name: String,
        target: String,
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

            let start = self.index;

            if let Some(symbol) = self.parse_symbol() {
                symbols.push(symbol);
            }

            if self.index == start {
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
        let mut brace_depth = 0usize;

        while let Some(guarded) = self.peek() {
            match guarded.token {
                Token::Semicolon
                if paren_depth == 0
                    && bracket_depth == 0
                    && brace_depth == 0 =>
                    {
                        self.advance();
                        break;
                    }

                Token::LBrace
                if paren_depth == 0 && bracket_depth == 0 =>
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
    DeclarationParser::new(tokens).parse()
}

struct DeclarationParser<'tok> {
    tokens: &'tok [GuardedToken<'tok>],
    index: usize,
}

impl<'tok> DeclarationParser<'tok> {
    fn new(tokens: &'tok [GuardedToken<'tok>]) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse(&mut self) -> Option<Symbol> {
        self.skip_decl_specifiers();

        let Some(token) = self.consume_token() else {
            return None;
        };

        match token.token {
            Token::Class => {
                self.parse_class_like_symbol(|name| SymbolKind::Class { name })
            },
            Token::Struct => {
                self.parse_class_like_symbol(|name| SymbolKind::Struct { name })
            },
            Token::Union => {
                self.parse_class_like_symbol(|name| SymbolKind::Union { name })
            },
            Token::Enum => {
                self.parse_enum_symbol()
            },
            Token::Using => {
                self.parse_using_declaration()
            },
            Token::Template => {
                self.parse_template_declaration()
            }
            _ => {
                None
            }
        }
    }

    fn parse_class_like_symbol(&mut self, constructor: impl FnOnce(String) -> SymbolKind) -> Option<Symbol> {
        if let Some(GuardedToken { guards, token: Token::Identifier(name) }) = self.consume_token().map(|token| token) {
            if matches!(self.peek_token()?.token, Token::Less | Token::DoubleColon) {
                // If we see this then we're likely creating a partial specialization, which we
                // can't export.
                return None;
            }

            Some(Symbol {
                guards: guards.to_vec(),
                kind: constructor(name.clone())
            })
        } else {
            None
        }
    }

    fn parse_enum_symbol(&mut self) -> Option<Symbol> {
        let Some(token) = self.peek_token() else {
            return None;
        };

        match token.token {
            Token::Identifier(name) => {
                return Some(Symbol {
                    guards: token.guards.to_vec(),
                    kind: SymbolKind::Enum {
                        name: name.clone(),
                        scoped: false
                    }
                })
            }
            Token::Class | Token::Struct => {
                self.advance();
            }
            _ => {
                return None;
            }
        }

        let Some(Token::Identifier(name)) = self.peek_token().map(|token| token.token) else {
            return None;
        };

        Some(Symbol {
            guards: token.guards.to_vec(),
            kind: SymbolKind::Enum {
                name: name.clone(),
                scoped: true
            }
        })
    }

    fn parse_using_declaration(&mut self) -> Option<Symbol> {
        let Some(token) = self.peek_token().map(|token| token) else {
            return None;
        };

        if *token.token == Token::Namespace {
            self.advance();
            let name = self.parse_scoped_identifier()?;

            if self.peek_token().is_some_and(|token| *token.token == Token::Equal) {
                self.advance();
                let target = self.parse_scoped_identifier()?;
                return Some(Symbol {
                    guards: token.guards.to_vec(),
                    kind: SymbolKind::NamespaceAlias { name, target }
                });
            }

            return Some(Symbol {
                guards: token.guards.to_vec(),
                kind: SymbolKind::UsingNamespace { name }
            });
        }

        let name = self.parse_scoped_identifier()?;

        if self.peek_token().is_some_and(|token| *token.token == Token::Equal) {
            return Some(Symbol {
                guards: token.guards.to_vec(),
                kind: SymbolKind::TypeAlias { name }
            });
        }

        Some(Symbol {
            guards: token.guards.to_vec(),
            kind: SymbolKind::UsingDeclaration { name }
        })
    }

    fn parse_template_declaration(&mut self) -> Option<Symbol> {
        self.skip_balanced_set(Token::Less, Token::Greater);

        // TODO: Skip the requires clause if one is present
        self.parse()
    }

    fn parse_scoped_identifier(&mut self) -> Option<String> {
        let mut name = String::new();

        while let Some(token) = self.peek_token() {
            match token.token {
                Token::Identifier(segment) => {
                    name.push_str(&segment);
                    self.advance();
                }
                Token::DoubleColon => {
                    self.advance();
                    name.push(':');
                    name.push(':');
                }
                _ => {
                    break;
                }
            }
        }

        if name.is_empty() {
            return None;
        }

        Some(name)
    }

    fn skip_decl_specifiers(&mut self) {
        loop {
            match self.peek_token().map(|token| token.token) {
                Some(Token::Inline) |
                    Some(Token::Static)
                | Some(Token::Extern)
                | Some(Token::Constexpr)
                | Some(Token::Friend)
                | Some(Token::Virtual) => {
                    self.advance();
                }
                Some(Token::Explicit) => {
                    self.advance();
                    self.skip_balanced_set(Token::LParen, Token::RParen);
                }
                _ => break
            }
        }
    }

    fn skip_balanced_set(&mut self, open: Token, close: Token) {
        if !self.peek_token().is_some_and(|token| *token.token == open) {
            return;
        }

        let mut depth = 1usize;
        self.advance();

        while let Some(token) = self.consume_token() {
            if *token.token == open {
                depth += 1;
            } else if *token.token == close {
                depth = depth.saturating_sub(1);

                if depth == 0 {
                    break;
                }
            }
        }
    }

    fn peek_token(&self) -> Option<GuardedToken<'tok>> {
        self.tokens.get(self.index).map(|token| token.clone())
    }

    fn consume_token(&mut self) -> Option<GuardedToken<'tok>> {
        self.tokens.get(self.index).map(|token| token.clone()).and_then(|token| {
            self.advance();
            Some(token)
        })
    }

    fn advance(&mut self) {
        self.index += 1;
    }
}

pub fn parse_symbols<'tok>(input: &'tok [GuardedToken<'tok>]) -> Result<Vec<Symbol>, SymbolError<'tok>>
{
    eprintln!("parse_symbols: starting");

    let result = SymbolParser::new(input).parse();

    eprintln!("parse_symbols: finished");

    Ok(result)
}