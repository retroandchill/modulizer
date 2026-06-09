use std::fmt;
use crate::parser::grammar::{GuardedToken, PreprocessorGuard, Token};
use std::fmt::Write;

#[derive(Debug)]
pub struct SymbolError {
    pub error: String
}

impl fmt::Display for SymbolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl std::error::Error for SymbolError{}

#[derive(Debug, Clone)]
pub struct CppNameSegment {
    pub name: String,
    pub has_template_args: bool
}

#[derive(Debug, Clone)]
pub struct Namespace {
    pub is_inline: bool,
    pub symbols: Vec<Symbol>,
}

impl Namespace {
    pub fn is_empty(&self) -> bool {
        if self.symbols.is_empty() {
            return true;
        }

        return self.symbols.iter().all(|symbol| {
            if let SymbolKind::Namespace(ns) = &symbol.kind {
                return ns.is_empty();
            }
            false
        })
    }
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Namespace(Namespace),
    ExportableSymbol,
    UsingNamespace,
    UsingDeclaration,
    NamespaceAlias(String)
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub guards: Vec<PreprocessorGuard>,
    pub kind: SymbolKind,
}

struct TokenParser<'tok> {
    tokens: &'tok [GuardedToken<'tok>],
    index: usize,
}

impl<'tok> TokenParser<'tok> {
    fn new(tokens: &'tok [GuardedToken<'tok>]) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse_scoped_identifier(&mut self) -> Option<&'tok [GuardedToken<'tok>]> {
        let start = self.index;
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

        Some(&self.tokens[start..self.index])
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

    fn peek(&self) -> Option<GuardedToken<'tok>> {
        self.tokens.get(self.index).map(|guarded| guarded.clone())
    }
    
    fn consume(&mut self) -> Option<GuardedToken<'tok>> {
        self.peek().map(|guarded| {
            self.advance();
            guarded
        })
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn is_at_end(&self) -> bool {
        self.index >= self.tokens.len()
    }
}

struct SymbolParser<'tok> {
    parser: TokenParser<'tok>
}

impl<'tok> SymbolParser<'tok> {
    fn new(tokens: &'tok [GuardedToken<'tok>]) -> Self {
        Self { parser: TokenParser::new(tokens) }
    }

    fn parse(mut self) -> Vec<Symbol> {
        self.parse_until(None)
    }

    fn parse_until(&mut self, end: Option<Token>) -> Vec<Symbol> {
        let mut symbols = Vec::new();

        while !self.parser.is_at_end() {
            if let Some(end) = &end {
                if self.parser.check(end) {
                    break;
                }
            }

            if self.parser.is_at_end() {
                break;
            }

            let start = self.parser.index;

            if let Some(symbol) = self.parse_symbol() {
                symbols.push(symbol);
            }

            if self.parser.index == start {
                self.parser.advance();
            }
        }

        symbols
    }

    fn parse_symbol(&mut self) -> Option<Symbol> {
        if self.parser.check(&Token::Inline) && self.parser.check_next(&Token::Namespace) {
            return self.parse_namespace();
        }

        if self.parser.check(&Token::Namespace) {
            return self.parse_namespace();
        }

        let chunk = self.collect_declaration_chunk();

        if chunk.is_empty() {
            return None;
        }

        classify_declaration_chunk(chunk)
    }

    fn parse_namespace(&mut self) -> Option<Symbol> {
        let first = self.parser.peek()?.clone();

        let is_inline = self.parser.match_token(&Token::Inline);

        if !self.parser.match_token(&Token::Namespace) {
            return None;
        }

        let mut name = self.parse_scoped_identifier()?;

        self.parser.skip_attributes();

        if self.parser.match_token(&Token::Equal) {
            if name.len() != 1 {
                return None;
            }

            let target = self.parse_scoped_identifier()?;
            return Some(Symbol {
                name: name.pop().unwrap(),
                guards: first.guards.to_vec(),
                kind: SymbolKind::NamespaceAlias(target.join("::"))
            })
        }

        if !self.parser.match_token(&Token::LBrace) {
            return Some(extract_namespace(first.guards, is_inline, name, Vec::new()))
        }

        let symbols = self.parse_until(Some(Token::RBrace));
        self.parser.match_token(&Token::RBrace);

        name.reverse();
        Some(extract_namespace(first.guards, is_inline, name, symbols))
    }

    fn collect_declaration_chunk(&mut self) -> &'tok [GuardedToken<'tok>] {
        let start = self.parser.index;

        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;

        while let Some(guarded) = self.parser.peek() {
            match guarded.token {
                Token::Semicolon
                if paren_depth == 0
                    && bracket_depth == 0
                    && brace_depth == 0 =>
                    {
                        self.parser.advance();
                        break;
                    }

                Token::LBrace
                if paren_depth == 0 && bracket_depth == 0 =>
                    {
                        brace_depth += 1;
                        self.parser.advance();

                        while brace_depth > 0 {
                            let Some(guarded) = self.parser.peek() else {
                                break;
                            };

                            match guarded.token {
                                Token::LBrace => brace_depth += 1,
                                Token::RBrace => brace_depth -= 1,
                                _ => {}
                            }

                            self.parser.advance();
                        }

                        if self.parser.check(&Token::Semicolon) {
                            self.parser.advance();
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
                    self.parser.advance();
                }
                Token::RParen => {
                    paren_depth = paren_depth.saturating_sub(1);
                    self.parser.advance();
                }
                Token::LBracket => {
                    bracket_depth += 1;
                    self.parser.advance();
                }
                Token::RBracket => {
                    bracket_depth = bracket_depth.saturating_sub(1);
                    self.parser.advance();
                }
                _ => {
                    self.parser.advance();
                }
            }
        }

        &self.parser.tokens[start..self.parser.index]
    }

    fn parse_scoped_identifier(&mut self) -> Option<Vec<String>> {
        self.parser.parse_scoped_identifier().map(|tokens| {
            let mut result = Vec::new();
            for token in tokens {
                if let Token::Identifier(name) = token.token {
                    result.push(name.clone());
                }
            }
            result
        })
    }
}

fn extract_namespace(guards: &[PreprocessorGuard], is_inline: bool, mut names: Vec<String>, symbols: Vec<Symbol>) -> Symbol {
    let name = names.pop().unwrap();
    if names.len() == 0 {
        return Symbol {
            name,
            guards: guards.to_vec(),
            kind: SymbolKind::Namespace(Namespace {
                is_inline,
                symbols,
            }),
        };
    }

    Symbol {
        name,
        guards: guards.to_vec(),
        kind: SymbolKind::Namespace(Namespace {
            is_inline,
            symbols: vec![extract_namespace(guards, is_inline, names, symbols)]
        }),
    }
}

fn classify_declaration_chunk(tokens: &[GuardedToken]) -> Option<Symbol> {
    DeclarationParser::new(tokens).parse()
}

struct DeclarationParser<'tok> {
    parser: TokenParser<'tok>,
}

impl<'tok> DeclarationParser<'tok> {
    fn new(tokens: &'tok [GuardedToken<'tok>]) -> Self {
        Self { parser: TokenParser::new(tokens) }
    }

    fn parse(&mut self) -> Option<Symbol> {
        self.skip_decl_specifiers();

        let Some(token) = self.parser.peek() else {
            return None;
        };

        match token.token {
            Token::Class => {
                self.parser.advance();
                self.parse_class_like_symbol()
            },
            Token::Struct => {
                self.parser.advance();
                self.parse_class_like_symbol()
            },
            Token::Union => {
                self.parser.advance();
                self.parse_class_like_symbol()
            },
            Token::Enum => {
                self.parser.advance();
                self.parse_enum_symbol()
            },
            Token::Using => {
                self.parser.advance();
                self.parse_using_declaration()
            },
            Token::Typedef => {
                self.parser.advance();
                self.parse_typedef_declaration()
            }
            Token::Concept => {
                self.parser.advance();
                self.parse_concept_declaration()
            }
            Token::Template => {
                self.parser.advance();
                self.parse_template_declaration()
            }
            _ => {
                self.parse_variable_or_function()
            }
        }
    }

    fn parse_class_like_symbol(&mut self) -> Option<Symbol> {
        if let Some(GuardedToken { guards, token: Token::Identifier(name) }) = self.parser.consume().map(|token| token) {
            if matches!(self.parser.peek()?.token, Token::Less | Token::DoubleColon) {
                // If we see this then we're likely creating a partial specialization, which we
                // can't export.
                return None;
            }

            Some(Symbol {
                name: name.clone(),
                guards: guards.to_vec(),
                kind: SymbolKind::ExportableSymbol
            })
        } else {
            None
        }
    }

    fn parse_enum_symbol(&mut self) -> Option<Symbol> {
        let Some(token) = self.parser.peek() else {
            return None;
        };

        match token.token {
            Token::Identifier(name) => {
                return Some(Symbol {
                    name: name.clone(),
                    guards: token.guards.to_vec(),
                    kind: SymbolKind::ExportableSymbol
                })
            }
            Token::Class | Token::Struct => {
                self.parser.advance();
            }
            _ => {
                return None;
            }
        }

        let Some(Token::Identifier(name)) = self.parser.peek().map(|token| token.token) else {
            return None;
        };

        Some(Symbol {
            name: name.clone(),
            guards: token.guards.to_vec(),
            kind: SymbolKind::ExportableSymbol
        })
    }

    fn parse_using_declaration(&mut self) -> Option<Symbol> {
        let Some(token) = self.parser.peek().map(|token| token) else {
            return None;
        };

        if *token.token == Token::Namespace {
            self.parser.advance();
            let name = self.parse_scoped_identifier()?;
            return Some(Symbol {
                name,
                guards: token.guards.to_vec(),
                kind: SymbolKind::UsingNamespace
            });
        }

        let name = self.parse_scoped_identifier()?;

        if self.parser.peek().is_some_and(|token| *token.token == Token::Equal) {
            return Some(Symbol {
                name,
                guards: token.guards.to_vec(),
                kind: SymbolKind::ExportableSymbol
            });
        }

        Some(Symbol {
            name,
            guards: token.guards.to_vec(),
            kind: SymbolKind::UsingDeclaration
        })
    }

    fn parse_typedef_declaration(&mut self) -> Option<Symbol> {
        self.skip_decl_specifiers();
        self.skip_type_specifier();

        let Some(token) = self.parser.peek() else {
            return None;
        };

        match token.token {
            Token::Identifier(name) => {
                Some(Symbol {
                    name: name.clone(),
                    guards: token.guards.to_vec(),
                    kind: SymbolKind::ExportableSymbol
                })
            }
            Token::LParen => {
                self.parser.advance();
                self.try_get_function_pointer_name()
                    .map(|name| Symbol {
                        name: name.clone(),
                        guards: token.guards.to_vec(),
                        kind: SymbolKind::ExportableSymbol
                    })
            }
            _ => None
        }
    }
    
    fn parse_concept_declaration(&mut self) -> Option<Symbol> {
        let Some(token) = self.parser.peek().map(|token| token) else {
            return None;
        };
        
        let Token::Identifier(name) = token.token else {
            return None;
        };
        
        Some(Symbol {
            name: name.clone(),
            guards: token.guards.to_vec(),
            kind: SymbolKind::ExportableSymbol
        })
    }

    fn parse_template_declaration(&mut self) -> Option<Symbol> {
        self.skip_template_arguments()?;

        self.skip_optional_requires_clause();
        self.parse()
    }

    fn parse_variable_or_function(&mut self) -> Option<Symbol> {
        let Some(token) = self.parser.peek().map(|token| token) else {
            return None;
        };

        match token.token {
            Token::Auto => {
                self.parser.advance();
            }
            Token::Decltype | Token::Typename | Token::DoubleColon | Token::Const | Token::Volatile | Token::Identifier(_)  => {
                self.skip_type_specifier();
            }
            _ => {
                return None;
            }
        }

        let Some(name_token) = self.parser.consume() else {
            return None;
        };

        match name_token.token {
            Token::Identifier(name) => {
                match self.parser.peek().map(|token| token.token) {
                    Some(Token::Equal) | Some(Token::Semicolon) | Some(Token::LParen) | Some(Token::LBracket) | None => {
                        Some(Symbol {
                            name: name.clone(),
                            guards: name_token.guards.to_vec(),
                            kind: SymbolKind::ExportableSymbol
                        })
                    }
                    _ => {
                        None
                    }
                }
            }
            Token::LParen => {
                self.try_get_function_pointer_name()
                        .map(|name: &String| Symbol {
                            name: name.clone(),
                            guards: name_token.guards.to_vec(),
                            kind: SymbolKind::ExportableSymbol
                        })
            }
            Token::Operator => {
                let start = self.parser.index;
                match self.parser.peek().map(|token| token.token) {
                    Some(Token::LParen) => {
                        self.parser.advance();
                    }
                    _ => {

                    }
                }

                loop {
                    let Some(symbol_token) = self.parser.peek() else {
                        return None;
                    };

                    if *symbol_token.token == Token::LParen {
                        break;
                    }
                    self.parser.advance();
                }

                let mut name = String::new();
                name.push_str("operator");
                for token in &self.parser.tokens[start..self.parser.index] {
                    name.write_fmt(format_args!("{}", token.token)).unwrap();
                }
                Some(Symbol {
                    name,
                    guards: name_token.guards.to_vec(),
                    kind: SymbolKind::ExportableSymbol
                })
            }
            _ => {
                None
            }
        }

    }

    fn try_get_function_pointer_name(&mut self) -> Option<&String> {
        let mut depth = 1usize;
        let mut is_function_pointer = self.parser.peek().map(|token| *token.token == Token::Star).unwrap_or(false);
        let mut function_pointer_name = None;
        loop {
            let Some(token) = self.parser.consume() else {
                return None;
            };
            match token.token {
                Token::LParen => {
                    depth += 1;
                }
                Token::RParen => {
                    depth = depth.saturating_sub(1);

                    if depth == 0 {
                        return function_pointer_name
                        ;
                    }
                },
                Token::DoubleColon => {
                    is_function_pointer = self.parser.peek().map(|token| *token.token == Token::Star).unwrap_or(false);
                }
                Token::Identifier(name) if is_function_pointer => {
                    function_pointer_name = Some(name);
                }
                _ => {}
            }
        }
    }

    fn skip_template_arguments(&mut self) -> Option<()> {
        if !matches!(self.parser.peek().map(|token| token.token), Some(Token::Less)) {
            return None;
        }

        self.parser.advance();
        let mut angle_depth = 1usize;
        let mut paren_depth = 0usize;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;

        while let Some(token) = self.parser.consume() {
            match token.token {
                Token::Less if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                    angle_depth += 1;
                }
                Token::Greater if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                    angle_depth = angle_depth.saturating_sub(1);

                    if angle_depth == 0 {
                        return Some(());
                    }
                }
                Token::LParen => {
                    paren_depth += 1;
                }
                Token::RParen => {
                    paren_depth = paren_depth.saturating_sub(1);
                }
                Token::LBracket => {
                    bracket_depth += 1;
                }
                Token::RBracket => {
                    bracket_depth = bracket_depth.saturating_sub(1);
                }
                Token::LBrace => {
                    brace_depth += 1;
                }
                Token::RBrace => {
                    brace_depth = brace_depth.saturating_sub(1);
                }
                _ => {}
            }
        }

        None
    }

    fn skip_optional_requires_clause(&mut self) {
        if !(self.parser.peek().is_some_and(|token| *token.token == Token::Requires)) {
            return;
        }
        
        self.parser.advance();
        loop {
            self.skip_requires_item();
            
            let Some(token) = self.parser.peek() else {
                return;
            };
            
            match token.token {
                Token::And | Token::Or => {
                    self.parser.advance();
                }
                _ => {
                    return;
                }
            }
        }
    }
    
    fn skip_requires_item(&mut self) {
        let Some(token) = self.parser.peek() else {
            return;
        };
        
        match token.token {
            Token::Requires => {
                self.parser.advance();
                self.skip_balanced_set(Token::LBrace, Token::RBrace);
            }
            Token::LParen => {
                self.skip_balanced_set(Token::LParen, Token::RParen);
            }
            Token::Typename |  Token::Template | Token::Identifier(_) => {
                self.skip_type_specifier();
            }
            _ => {
            }
        }
    }
    
    fn skip_type_specifier(&mut self) {
        loop {
            let Some(token) = self.parser.peek() else {
                return;
            };

            match token.token {
                Token::Const | Token::Volatile => {
                    self.parser.advance();
                }
                _ => {
                    break;
                }
            }
        }

        self.parser.match_token(&Token::DoubleColon);

        loop {
            let Some(token) = self.parser.peek() else {
                return;
            };

            match token.token {
                Token::Typename |  Token::Template => {
                    self.parser.advance()
                },
                Token::Decltype => {
                    self.parser.advance();
                    self.skip_balanced_set(Token::LParen, Token::RParen);

                    if !self.parser.match_token(&Token::DoubleColon) {
                        break;
                    }
                }
                Token::Identifier(_) => {
                    self.parser.advance();
                    self.skip_template_arguments();
                    
                    if !self.parser.match_token(&Token::DoubleColon) {
                        break;
                    }
                }
                _ => {
                    break;
                }
            }
        }

        loop {
            let Some(token) = self.parser.peek() else {
                return;
            };

            match token.token {
                Token::Star | Token::Amp | Token::Const | Token::Volatile => {
                    self.parser.advance();
                }
                _ => {
                    break;
                }
            }
        }
    }

    fn parse_scoped_identifier(&mut self) -> Option<String> {
        let mut name = String::new();

        while let Some(token) = self.parser.peek() {
            match token.token {
                Token::Identifier(segment) => {
                    name.push_str(&segment);
                    self.parser.advance();
                }
                Token::DoubleColon => {
                    self.parser.advance();
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
            self.parser.skip_attributes();
            match self.parser.peek().map(|token| token.token) {
                Some(Token::Inline) |
                    Some(Token::Static)
                | Some(Token::Extern)
                | Some(Token::Constexpr)
                | Some(Token::Consteval)
                | Some(Token::Constinit)
                | Some(Token::Friend)
                | Some(Token::Virtual) => {
                    self.parser.advance();
                }
                Some(Token::Explicit) => {
                    self.parser.advance();
                    self.skip_balanced_set(Token::LParen, Token::RParen);
                }
                _ => break
            }
        }
    }

    fn skip_balanced_set(&mut self, open: Token, close: Token) {
        if !self.parser.peek().is_some_and(|token| *token.token == open) {
            return;
        }

        let mut depth = 1usize;
        self.parser.advance();

        while let Some(token) = self.parser.consume() {
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
}

pub fn parse_symbols<'tok>(input: &'tok [GuardedToken<'tok>]) -> Result<Vec<Symbol>, SymbolError>
{
    eprintln!("parse_symbols: starting");

    let result = SymbolParser::new(input).parse();

    eprintln!("parse_symbols: finished");

    Ok(result)
}


#[cfg(test)]
mod test {
    use std::assert_matches;
    use logos::Logos;
    use super::*;

    fn lex(source: &str) -> Vec<Token> {
        Token::lexer(source)
            .filter_map(|result| {
                result.ok()
            })
            .collect()
    }

    fn to_guarded_tokens(tokens: &[Token]) -> Vec<GuardedToken<'_>> {
        tokens.iter()
            .filter(|token| !token.is_trivial())
            .map(|token| {
                GuardedToken {
                    token,
                    guards: &[]
                }
            })
            .collect()
    }

    fn assert_declarations(actual: &[Symbol], expected: &[&str]) {
        let actual_names: Vec<_> = actual.iter().map(|symbol| &symbol.name).collect();
        assert_eq!(actual_names, expected);
        assert!(actual.iter().all(|symbol| matches!(&symbol.kind, SymbolKind::ExportableSymbol)));
    }

    #[test]
    fn can_parse_namespace_symbols() {
        let code = "namespace Engine {}

                          namespace Engine::Core {}

                          namespace EC = Engine::Core;

                          using namespace Engine;
                          using Engine::Core;";

        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 5);

        assert_eq!(symbols[0].name, "Engine");
        assert_matches!(&symbols[0].kind, SymbolKind::Namespace(namespace) if namespace.symbols.is_empty());

        assert_eq!(symbols[1].name, "Engine");
        assert_matches!(&symbols[1].kind, SymbolKind::Namespace(namespace) if namespace.symbols.len() == 1 && namespace.symbols[0].name == "Core");

        assert_eq!(symbols[2].name, "EC");
        assert_matches!(&symbols[2].kind, SymbolKind::NamespaceAlias(target) if target == "Engine::Core");

        assert_eq!(symbols[3].name, "Engine");
        assert_matches!(&symbols[3].kind, SymbolKind::UsingNamespace);

        assert_eq!(symbols[4].name, "Engine::Core");
        assert_matches!(&symbols[4].kind, SymbolKind::UsingDeclaration);
    }

    #[test]
    fn can_parse_type_aliases() {
        let code = "typedef int Int32;
                         typedef const char* CString;

                         typedef void (*Callback)(int);
                         typedef int (*MathFn)(int, int);

                         using String = std::string;
                         using IntList = std::vector<int>;

                         using Predicate = bool(*)(int);";

        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "Int32",
                "CString",
                "Callback",
                "MathFn",
                "String",
                "IntList",
                "Predicate"
            ]
        );
    }

    #[test]
    fn can_parse_enums() {
        let code = "enum Color
                         {
                             Red,
                             Green,
                             Blue
                         };

                         enum class Direction
                         {
                             North,
                             South,
                             East,
                             West
                         };

                         enum class Byte : unsigned char
                         {
                             A,
                             B
                         };";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "Color",
                "Direction",
                "Byte"
            ]
        );
    }

    #[test]
    fn can_parse_structs_classes_and_unions() {
        let code = "struct Empty {};

                          struct Base
                          {
                              int Value;
                          };

                          struct Derived final : Base
                          {
                          };

                          class Actor
                          {
                          public:
                              void Tick();
                          };

                          union Variant
                          {
                              int IntValue;
                              float FloatValue;
                          };";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "Empty",
                "Base",
                "Derived",
                "Actor",
                "Variant"
            ]
        );
    }

    #[test]
    fn can_parse_variables() {
        let code = "int GlobalInt;

                          extern int ExternalInt;

                          inline constexpr int CompileTimeValue = 42;

                          const char* GlobalString;";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "GlobalInt",
                "ExternalInt",
                "CompileTimeValue",
                "GlobalString"
            ]
        );
    }

    #[test]
    fn can_parse_array_variables() {
        let code = "int FixedArray[10];

                          extern float LookupTable[256];";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "FixedArray",
                "LookupTable"
            ]
        );
    }

    #[test]
    fn can_parse_references_and_pointer_variables() {
        let code = "int* Ptr;

                          const int* ConstPtr;

                          int& Ref = GlobalInt;

                          int&& RValueRef = 42;";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "Ptr",
                "ConstPtr",
                "Ref",
                "RValueRef"
            ]
        );
    }

    #[test]
    fn can_parse_function_declarations() {
        let code = "void SimpleFunction();

                          int Add(int a, int b);

                          const char* GetName();

                          void NoexceptFunction() noexcept;

                          [[nodiscard]] int Compute();";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "SimpleFunction",
                "Add",
                "GetName",
                "NoexceptFunction",
                "Compute"
            ]
        );
    }

    #[test]
    fn can_parse_function_pointer_variables() {
        let code = "void (*GlobalCallback)(int);

                          int (*GlobalMathFn)(int, int);

                          void (*SignalHandlers[8])(int);";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "GlobalCallback",
                "GlobalMathFn",
                "SignalHandlers"
            ]
        );
    }

    #[test]
    fn can_parse_functions_returning_function_pointers() {
        let code = "int (*GetMathFunction())(int, int);";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "GetMathFunction"
            ]
        );
    }

    #[test]
    fn can_parse_functions_returning_function_references() {
        let code = "void (&CallbackRef)(int) = *GlobalCallback;";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "CallbackRef"
            ]
        );
    }

    #[test]
    fn can_parse_pointers_to_members() {
        let code = "struct MemberExample
                          {
                              int Value;

                              void Method();
                          };

                          int MemberExample::*ValuePtr;

                          void (MemberExample::*MethodPtr)();";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "MemberExample",
                "ValuePtr",
                "MethodPtr"
            ]
        );
    }

    #[test]
    fn can_parse_template_declarations() {
        let code = "template<typename T>
                          struct Vector
                          {
                          };

                          template<typename T, typename U>
                          class Pair
                          {
                          };

                          template<typename T>
                          using Ptr = T*;

                          template<typename T>
                          inline constexpr bool IsVoid = false;

                          template<typename T>
                          T Max(T a, T b);";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "Vector",
                "Pair",
                "Ptr",
                "IsVoid",
                "Max"
            ]
        );
    }

    #[test]
    fn can_parse_concept_declarations() {
        let code = "template<typename T>
                          concept Incrementable = requires(T t)
                          {
                              ++t;
                          };";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "Incrementable"
            ]
        );
    }

    #[test]
    fn can_parse_types_with_friend_functions() {
        let code = "class FriendOwner
                          {
                              friend class FriendClass;

                              friend void FriendFunction();
                          };";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "FriendOwner"
            ]
        );
    }

    #[test]
    fn can_parse_nested_types() {
        let code = "class Outer
                          {
                          public:
                              struct Inner
                              {
                                  enum class State
                                  {
                                      Idle,
                                      Running
                                  };
                              };
                          };";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "Outer"
            ]
        );
    }

    #[test]
    fn can_parse_declarations_with_attributes() {
        let code = "[[nodiscard]]int ImportantFunction();
                         [[deprecated]]void OldFunction();";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "ImportantFunction",
                "OldFunction"
            ]
        );
    }

    #[test]
    fn can_parse_trailing_return_types() {
        let code = "auto ModernFunction() -> int;
                         auto Factory() -> Actor*;";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "ModernFunction",
                "Factory"
            ]
        );
    }

    #[test]
    fn can_parse_variables_with_inferred_types() {
        let code = "auto AutoValue = 42;";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(
            &symbols,
            &[
                "AutoValue"
            ]
        );
    }

    #[test]
    fn can_parse_inline_namespace() {
        let code = "inline namespace V1
                         {
                             struct VersionedType {};
                         }";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_eq!(symbols.len(), 1);
        let Symbol { name, kind: SymbolKind::Namespace(namespace), .. } = &symbols[0] else {
            assert!(false, "Expected a namespace symbol");
            unreachable!()
        };
        assert_eq!(*name, "V1");
        assert_eq!(namespace.symbols.len(), 1);
        assert_eq!(&namespace.symbols[0].name, "VersionedType");
        assert!(namespace.is_inline);
        assert_matches!(&namespace.symbols[0].kind, SymbolKind::ExportableSymbol);
    }
}