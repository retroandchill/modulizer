use crate::parser::grammar::{GuardedToken, PreprocessorGuard, Token};
use crate::parser::structure::{Delimiter, TokenGroup, TokenNode, collect_token_nodes, strip_attributes};
use chumsky::container::Seq;
use chumsky::input::{BorrowInput, ValueInput};
use chumsky::{IterParser, Parser};
use itertools::Itertools;
use regex::bytes::Replacer;
use std::fmt;
use std::fmt::Write;
use std::rc::Rc;
use arraystring::ArrayString;
use arraystring::typenum::U25;
use ustr::Ustr;

type OperatorOverloadName = ArrayString<U25>;

#[derive(Debug)]
pub struct SymbolError {
    pub error: String,
}

impl fmt::Display for SymbolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl std::error::Error for SymbolError {}

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
        });
    }
}

#[derive(Debug, Clone)]
pub enum SymbolKind {
    Namespace(Namespace),
    ExportableSymbol,
    UsingNamespace,
    UsingDeclaration,
    NamespaceAlias(String),
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: Ustr,
    pub guards: Rc<[PreprocessorGuard]>,
    pub kind: SymbolKind,
}

#[derive(Debug, Clone)]
struct SymbolParser<'a> {
    tokens: &'a [TokenNode],
    index: usize,
}

impl<'a> SymbolParser<'a> {
    fn new(tokens: &'a [TokenNode]) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse(mut self) -> Vec<Symbol> {
        let mut symbols = Vec::new();

        while !self.is_at_end() {
            if let Some(symbol) = self.parse_symbol() {
                symbols.push(symbol);
            } else {
                self.advance();
            }
        }

        symbols
    }

    fn parse_symbol(&mut self) -> Option<Symbol> {
        match self.try_peak_token()?.token {
            Token::Inline if self.check_next_token(Token::Namespace).is_some() => {
                self.advance();
                self.parse_namespace(true)
            }
            Token::Namespace => self.parse_namespace(false),
            Token::Class => self.parse_class(),
            Token::Struct => self.parse_struct(),
            Token::Union => self.parse_union(),
            Token::Enum => self.parse_enum(),
            Token::Using => self.parse_using(),
            Token::Typedef => self.parse_typedef(),
            Token::Template => self.parse_template(),
            _ => self.parse_variable_or_function(),
        }
    }

    fn parse_namespace(&mut self, is_inline: bool) -> Option<Symbol> {
        let start = self.expect_token(Token::Namespace)?;

        if let Some(Token::Identifier(name)) = self.try_peak_token().map(|guarded| guarded.token)
            && self.check_next_token(Token::Equal).is_some()
        {
            self.advance();
            self.expect_token(Token::Equal)?;
            let target = self
                .parse_qualified_name()?
                .iter()
                .map(|s| s.as_str())
                .join("::");
            return Some(Symbol {
                name: name.clone(),
                guards: start.guards,
                kind: SymbolKind::NamespaceAlias(target),
            });
        }

        let Some(mut names) = self.parse_qualified_name() else {
            self.expect_group(Delimiter::Braces)?;
            return None;
        };
        names.reverse();

        let group = self.expect_group(Delimiter::Braces)?;
        let sub_parser = SymbolParser::new(&group.children);
        let children = sub_parser.parse();

        Some(extract_namespace(start.guards, is_inline, names, children))
    }

    fn parse_class(&mut self) -> Option<Symbol> {
        self.skip_declaration_qualifiers();
        let declaration = self.expect_token(Token::Class)?;
        self.parse_class_or_struct(declaration)
    }

    fn parse_struct(&mut self) -> Option<Symbol> {
        self.skip_declaration_qualifiers();
        let declaration = self.expect_token(Token::Struct)?;
        self.parse_class_or_struct(declaration)
    }

    fn parse_class_or_struct(&mut self, declaration: GuardedToken) -> Option<Symbol> {
        let name = match self.try_peak_token()?.token {
            Token::Identifier(name) => {
                self.advance();
                Some(name)
            }
            _ => None,
        };

        // Skip over the base class list if present
        if self.check_token(Token::DoubleColon).is_some() || self.check_token(Token::Less).is_some()
        {
            // This is probably a partial specialization, which can't be exported
            return None;
        }
        self.skip_optional_base_class_list();
        self.skip_optional_scope();
        self.expect_token(Token::Semicolon)?;

        name.map(|name| Symbol {
            name: name.clone(),
            guards: declaration.guards,
            kind: SymbolKind::ExportableSymbol,
        })
    }

    fn skip_optional_base_class_list(&mut self) {
        while !self.check_token(Token::Semicolon).is_some()
            && !self.check_group(Delimiter::Braces).is_some()
        {
            self.advance();
        }
    }

    fn parse_union(&mut self) -> Option<Symbol> {
        self.skip_declaration_qualifiers();
        let declaration = self.expect_token(Token::Union)?;
        let name = match self.try_peak_token()?.token {
            Token::Identifier(name) => {
                self.advance();
                Some(name)
            }
            _ => None,
        };

        self.skip_optional_scope();
        self.expect_token(Token::Semicolon)?;

        name.map(|name| Symbol {
            name: name.clone(),
            guards: declaration.guards,
            kind: SymbolKind::ExportableSymbol,
        })
    }

    fn parse_enum(&mut self) -> Option<Symbol> {
        self.skip_declaration_qualifiers();
        let declaration = self.expect_token(Token::Enum)?;

        if self.check_token(Token::Class).is_some() || self.check_token(Token::Struct).is_some() {
            self.advance();
        }
        self.parse_class_or_struct(declaration)
    }
    fn parse_using(&mut self) -> Option<Symbol> {
        self.skip_declaration_qualifiers();
        self.expect_token(Token::Using)?;

        let next = self.try_peak_token()?;
        match next.token {
            Token::Namespace => {
                self.advance();
                let names = self.parse_qualified_name()?;
                self.expect_token(Token::Semicolon)?;
                Some(Symbol {
                    name: Ustr::from(names.iter().map(|s| s.as_str()).join("::").as_str()),
                    guards: next.guards,
                    kind: SymbolKind::UsingNamespace,
                })
            }
            Token::Identifier(name) => {
                if self.check_next_token(Token::Equal).is_some() {
                    self.skip_until_semicolon();
                    Some(Symbol {
                        name: name.clone(),
                        guards: next.guards,
                        kind: SymbolKind::ExportableSymbol,
                    })
                } else {
                    let names = self.parse_qualified_name()?;
                    self.expect_token(Token::Semicolon)?;
                    Some(Symbol {
                        name: Ustr::from(names.iter().map(|s| s.as_str()).join("::").as_str()),
                        guards: next.guards,
                        kind: SymbolKind::UsingDeclaration,
                    })
                }
            }
            _ => None,
        }
    }

    fn parse_typedef(&mut self) -> Option<Symbol> {
        self.skip_declaration_qualifiers();
        let declaration = self.expect_token(Token::Typedef)?;

        let is_auto = self.parse_type_specifier()?;
        let (name, _, _) = self.parse_variable_or_function_name(is_auto)?;

        Some(Symbol {
            name,
            guards: declaration.guards,
            kind: SymbolKind::ExportableSymbol,
        })
    }

    fn parse_concept(&mut self) -> Option<Symbol> {
        let declaration = self.expect_token(Token::Concept)?;

        let name = match self.try_peak_token()?.token {
            Token::Identifier(name) => {
                self.advance();
                Some(name)
            }
            _ => None,
        }?;

        self.expect_token(Token::Equal)?;
        self.skip_until_semicolon();

        Some(Symbol {
            name: name.clone(),
            guards: declaration.guards,
            kind: SymbolKind::ExportableSymbol,
        })
    }

    fn parse_template(&mut self) -> Option<Symbol> {
        self.expect_token(Token::Template)?;
        self.parse_template_parameters()?;
        self.skip_requires_clause()?;
        match self.try_peak_token()?.token {
            Token::Class => self.parse_class(),
            Token::Struct => self.parse_struct(),
            Token::Union => self.parse_union(),
            Token::Enum => self.parse_enum(),
            Token::Using => self.parse_using(),
            Token::Concept => self.parse_concept(),
            _ => self.parse_variable_or_function(),
        }
    }

    fn parse_template_parameters(&mut self) -> Option<()> {
        self.expect_token(Token::Less)?;
        let mut depth = 1usize;
        while depth > 0 {
            match self.try_peak_token().map(|token| token.token) {
                Some(Token::Less) => depth += 1,
                Some(Token::Greater) => depth = depth.saturating_sub(1),
                _ => {}
            }
            self.advance();
        }

        Some(())
    }

    fn skip_requires_clause(&mut self) -> Option<()> {
        if self.expect_token(Token::Requires).is_none() {
            return Some(());
        }

        loop {
            match self.peek()? {
                TokenNode::Token(token) => {
                    match token.token {
                        Token::Identifier(_) | Token::DoubleColon | Token::Decltype => {
                            self.parse_templatable_identifier()?;
                        }
                        Token::Requires => {
                            self.advance();
                            self.expect_group(Delimiter::Parentheses);
                            self.expect_group(Delimiter::Braces)?;
                        }
                        _ => return None,
                    }
                }
                TokenNode::Group(group) => {
                    if group.delimiter == Delimiter::Parentheses {
                        self.advance();
                    }
                    else {
                        return None;
                    }
                }
            }

            match self.try_peak_token().map(|token| token.token) {
                Some(Token::And | Token::Or) => self.advance(),
                _ => break
            }
        }

        Some(())
    }

    fn parse_variable_or_function(&mut self) -> Option<Symbol> {
        self.skip_declaration_qualifiers();
        let is_auto = self.parse_type_specifier()?;
        let (name, guards, is_function)  = self.parse_variable_or_function_name(is_auto)?;

        if is_function {
            self.parse_function_body(is_auto)?;
        }
        else if self.check_token(Token::Semicolon).is_some() {
            self.advance();
        } else if self.check_token(Token::Equal).is_some() {
            self.skip_until_semicolon();
        }
        else {
            return None;
        }

        Some(Symbol {
            name,
            guards,
            kind: SymbolKind::ExportableSymbol,
        })
    }

    fn parse_variable_or_function_name(&mut self, is_auto: bool) -> Option<(Ustr, Rc<[PreprocessorGuard]>, bool)> {
        let node = self.peek()?;
        match node {
            TokenNode::Token(identifier) => {
                match identifier.token {
                    Token::Identifier(name) => {
                        self.advance();
                        if self.check_group(Delimiter::Brackets).is_some() {
                            self.advance();
                            Some((name, identifier.guards, false))
                        } else if self.check_group(Delimiter::Parentheses).is_some() {
                            self.advance();

                            if self.expect_token(Token::Noexcept).is_some() {
                                self.expect_group(Delimiter::Parentheses);

                            }

                            Some((name, identifier.guards, true))
                        }
                        else {
                            Some((name, identifier.guards, false))
                        }
                    }
                    Token::Operator => {
                        let name = self.parse_operator_overload_name()?;
                        self.expect_group(Delimiter::Parentheses)?;
                        Some((name, identifier.guards, true))
                    }
                    _ => None,
                }
            }
            TokenNode::Group(group) => {
                match group.delimiter {
                    Delimiter::Parentheses => {
                        self.advance();
                        let mut sub_parser = SymbolParser::new(&group.children);
                        let identifier = sub_parser.try_peak_token()?;
                        match identifier.token {
                            Token::Star | Token::Amp | Token::And => {
                                sub_parser.advance();
                                sub_parser.skip_optional_cv_qualifiers();
                            }
                            Token::Identifier(_) => {
                                sub_parser.parse_templatable_identifier()?;
                                sub_parser.expect_token(Token::DoubleColon)?;
                                sub_parser.expect_token(Token::Star)?;
                            }
                            _ => return None,
                        }

                        let dec_info = sub_parser.parse_variable_or_function_name(is_auto)?;

                        self.expect_group(Delimiter::Parentheses)?;

                        if self.expect_token(Token::Noexcept).is_some() {
                            self.expect_group(Delimiter::Parentheses);

                        }

                        if is_auto {
                            if self.expect_token(Token::Arrow).is_some() {
                                self.parse_type_specifier()?;
                            }
                        }

                        Some(dec_info)
                    }
                    _ => return None
                }
            }
        }
    }

    fn parse_type_specifier(&mut self) -> Option<bool> {
        self.skip_optional_cv_qualifiers();
        let is_auto;
        if let Some(auto) = self.parse_fundamental_type() {
            is_auto = auto;
        }
        else {
            self.expect_token(Token::Typename);
            self.parse_templatable_identifier()?;
            is_auto = false;
        }
        self.skip_cv_ref_qualifiers();

        let mut pointer_to_member_check = self.clone();
        if let Some(_) = pointer_to_member_check.parse_templatable_identifier() {
            if pointer_to_member_check.expect_token(Token::DoubleColon).is_some() && pointer_to_member_check.check_token(Token::Star).is_some() {
                pointer_to_member_check.advance();
                *self = pointer_to_member_check;
            }
        }

        Some(is_auto)
    }

    fn parse_templatable_identifier(&mut self) -> Option<()> {
        // A type can optionally start with one of these
        self.expect_token(Token::DoubleColon);
        let mut template_allowed = false;
        loop {
            match self.try_peak_token()?.token {
                Token::Identifier(_) => {
                    template_allowed = true;
                    self.advance();
                    self.parse_template_parameters();
                }
                Token::Decltype => {
                    template_allowed = true;
                    self.advance();
                    self.expect_group(Delimiter::Parentheses)?;
                }
                Token::Template if template_allowed => {
                    self.advance();
                    continue;
                }
                _ => return None,
            }

            if !self.check_token(Token::DoubleColon).is_some() || !self.check_next_token(Token::Star).is_none() {
                break;
            }
            self.advance();
        }

        Some(())
    }

    fn skip_optional_cv_qualifiers(&mut self) {
        loop {
            match self.try_peak_token().map(|token| token.token) {
                Some(Token::Const | Token::Volatile) => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn skip_cv_ref_qualifiers(&mut self) {
        loop {
            match self.try_peak_token().map(|token| token.token) {
                Some(Token::Star | Token::Amp | Token::And | Token::Const | Token::Volatile) => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn parse_fundamental_type(&mut self) -> Option<bool> {
        match self.try_peak_token().map(|token| token.token) {
            Some(Token::Auto) => {
                self.advance();
                Some(true)
            }
            Some(
                Token::Void
                | Token::Bool
                | Token::Int
                | Token::Float
                | Token::Double
                | Token::Char
                | Token::WChar
                | Token::Char8
                | Token::Char16
                | Token::Char32,
            ) => {
                self.advance();
                Some(false)
            }
            Some(Token::Signed | Token::Unsigned) => {
                self.advance();
                self.parse_sign_modified_type();
                Some(false)
            }
            Some(Token::Short | Token::Long | Token::LongLong) => {
                self.advance();
                self.expect_token(Token::Int);
                Some(false)
            }
            _ => None,
        }
    }

    fn parse_sign_modified_type(&mut self) {
        match self.try_peak_token().map(|token| token.token) {
            Some(Token::Int | Token::Char) => {
                self.advance();
            }
            Some(Token::Short | Token::Long | Token::LongLong) => {
                self.advance();
            }
            _ => {}
        }
    }

    fn skip_declaration_qualifiers(&mut self) {
        loop {
            match self.try_peak_token().map(|token| token.token) {
                Some(
                    Token::Constexpr
                    | Token::Consteval
                    | Token::Constinit
                    | Token::Inline
                    | Token::Static,
                ) => {
                    self.advance();
                }
                Some(Token::Extern) => {
                    self.advance();
                    if let Some(Token::StringLiteral(_)) =
                        self.try_peak_token().map(|token| token.token)
                    {
                        self.advance();
                    }
                }
                _ => break,
            }
        }
    }

    fn parse_operator_overload_name(&mut self) -> Option<Ustr> {
        self.expect_token(Token::Operator)?;
        match self.peek()? {
            TokenNode::Token(guarded) => {
                match guarded.token {
                    Token::New => {
                        self.advance();
                        if self.check_group(Delimiter::Brackets).is_some_and(|group| group.children.is_empty()) {
                            self.advance();
                            return Some(Ustr::from("operator new[]"));
                        }

                        Some(Ustr::from("operator new"))
                    }
                    Token::Delete => {
                        self.advance();
                        if self.check_group(Delimiter::Brackets).is_some_and(|group| group.children.is_empty()) {
                            self.advance();
                            return Some(Ustr::from("operator delete[]"));
                        }

                        Some(Ustr::from("operator delete"))
                    }
                    Token::CoAwait => {
                        self.advance();
                        Some(Ustr::from("operator co_await"))
                    }
                    _ => {
                        let mut buffer = OperatorOverloadName::new();
                        buffer.push_str("operator");
                        loop {
                            let next = self.peek()?;
                            match next {
                                TokenNode::Token(token) => {
                                    self.advance();
                                    buffer.write_fmt(format_args!("{}", token.token)).ok()?;
                                }
                                TokenNode::Group(group) => {
                                    if group.delimiter == Delimiter::Parentheses {
                                        break;
                                    }
                                    else {
                                        return None;
                                    }
                                }
                            }
                        }

                        Some(Ustr::from( buffer.as_str() ) )
                    }
                }
            }
            TokenNode::Group(group) => {
                if !group.children.is_empty() {
                    return None;
                }

                match group.delimiter {
                    Delimiter::Parentheses => {
                        self.advance();
                        Some(Ustr::from("operator()"))
                    },
                    Delimiter::Brackets => {
                        self.advance();
                        Some(Ustr::from("operator[]"))
                    },
                    Delimiter::Braces => {
                        None
                    }
                }
            }
        }
    }

    fn parse_function_body(&mut self, is_auto: bool) -> Option<()> {
        if is_auto {
            if self.expect_token(Token::Arrow).is_some() {
                self.parse_type_specifier()?;
            }
        }

        self.skip_requires_clause()?;

        if self.check_token(Token::Semicolon).is_some()
            || self.check_group(Delimiter::Braces).is_some()
        {
            self.advance();
        } else {
            return None;
        }

        Some(())
    }

    fn skip_optional_scope(&mut self) {
        if self.check_group(Delimiter::Braces).is_some() {
            self.advance();
        }
    }

    fn skip_until_semicolon(&mut self) {
        while !self.is_at_end() && !self.check_token(Token::Semicolon).is_some() {
            self.advance();
        }

        if !self.is_at_end() {
            self.advance();
        }
    }

    fn parse_qualified_name(&mut self) -> Option<Vec<Ustr>> {
        let mut parts = Vec::new();

        let Token::Identifier(name) = self.try_peak_token()?.token else {
            return None;
        };

        parts.push(name.clone());
        self.advance();

        while self.expect_token(Token::DoubleColon).is_some() {
            let Some(guarded) = self.try_peak_token() else {
                break;
            };

            let Token::Identifier(name) = guarded.token else {
                break;
            };

            parts.push(name.clone());
            self.advance();
        }

        Some(parts)
    }

    fn expect_token(&mut self, expected: Token) -> Option<GuardedToken> {
        self.check_token(expected).map(|guarded| {
            self.advance();
            guarded
        })
    }

    fn expect_group(&mut self, expected: Delimiter) -> Option<TokenGroup> {
        self.check_group(expected).map(|group| {
            self.advance();
            group
        })
    }

    fn check_token(&self, expected: Token) -> Option<GuardedToken> {
        self.try_peak_token()
            .filter(|guarded| guarded.token == expected)
    }

    fn check_next_token(&self, expected: Token) -> Option<GuardedToken> {
        self.tokens
            .get(self.index + 1)
            .and_then(|token| token.try_get_token())
            .filter(|guarded| guarded.token == expected)
    }

    fn check_group(&self, expected: Delimiter) -> Option<TokenGroup> {
        self.try_peak_group()
            .filter(|group| group.delimiter == expected)
    }

    fn peek(&self) -> Option<TokenNode> {
        self.tokens.get(self.index).map(|guarded| guarded.clone())
    }

    fn try_peak_token(&self) -> Option<GuardedToken> {
        self.peek().and_then(|token| token.try_get_token())
    }

    fn try_peak_group(&self) -> Option<TokenGroup> {
        self.peek().and_then(|token| token.try_get_group())
    }

    fn advance(&mut self) {
        self.index += 1;
    }

    fn is_at_end(&self) -> bool {
        self.index >= self.tokens.len()
    }
}

fn extract_namespace(
    guards: Rc<[PreprocessorGuard]>,
    is_inline: bool,
    mut names: Vec<Ustr>,
    symbols: Vec<Symbol>,
) -> Symbol {
    let name = names.pop().unwrap();
    if names.len() == 0 {
        return Symbol {
            name,
            guards,
            kind: SymbolKind::Namespace(Namespace { is_inline, symbols }),
        };
    }

    Symbol {
        name,
        guards: guards.clone(),
        kind: SymbolKind::Namespace(Namespace {
            is_inline,
            symbols: vec![extract_namespace(guards, is_inline, names, symbols)],
        }),
    }
}

pub fn parse_symbols<'tok>(input: &'tok [GuardedToken]) -> Result<Vec<Symbol>, SymbolError> {
    eprintln!("parse_symbols: starting");

    let nodes = collect_token_nodes(input);
    let nodes = strip_attributes(nodes);
    let result = SymbolParser::new(&nodes).parse();

    eprintln!("parse_symbols: finished");

    Ok(result)
}

#[cfg(test)]
mod test {
    use super::*;
    use logos::Logos;
    use std::assert_matches;

    fn lex(source: &str) -> Vec<Token> {
        Token::lexer(source)
            .filter_map(|result| result.ok())
            .collect()
    }

    fn to_guarded_tokens(tokens: &[Token]) -> Vec<GuardedToken> {
        tokens
            .iter()
            .filter(|token| !token.is_trivial())
            .map(|token| GuardedToken {
                token: token.clone(),
                guards: Rc::new([]),
            })
            .collect()
    }

    fn assert_declarations(actual: &[Symbol], expected: &[&str]) {
        let actual_names: Vec<_> = actual.iter().map(|symbol| symbol.name.as_str()).collect();
        assert_eq!(actual_names, expected);
        assert!(
            actual
                .iter()
                .all(|symbol| matches!(&symbol.kind, SymbolKind::ExportableSymbol))
        );
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
                "Predicate",
            ],
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
        assert_declarations(&symbols, &["Color", "Direction", "Byte"]);
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
        assert_declarations(&symbols, &["Empty", "Base", "Derived", "Actor", "Variant"]);
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
                "GlobalString",
            ],
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
        assert_declarations(&symbols, &["FixedArray", "LookupTable"]);
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
        assert_declarations(&symbols, &["Ptr", "ConstPtr", "Ref", "RValueRef"]);
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
                "Compute",
            ],
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
            &["GlobalCallback", "GlobalMathFn", "SignalHandlers"],
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
        assert_declarations(&symbols, &["GetMathFunction"]);
    }

    #[test]
    fn can_parse_functions_returning_function_references() {
        let code = "void (&CallbackRef)(int) = *GlobalCallback;";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(&symbols, &["CallbackRef"]);
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
        assert_declarations(&symbols, &["MemberExample", "ValuePtr", "MethodPtr"]);
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
        assert_declarations(&symbols, &["Vector", "Pair", "Ptr", "IsVoid", "Max"]);
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
        assert_declarations(&symbols, &["Incrementable"]);
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
        assert_declarations(&symbols, &["FriendOwner"]);
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
        assert_declarations(&symbols, &["Outer"]);
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
        assert_declarations(&symbols, &["ImportantFunction", "OldFunction"]);
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
        assert_declarations(&symbols, &["ModernFunction", "Factory"]);
    }

    #[test]
    fn can_parse_variables_with_inferred_types() {
        let code = "auto AutoValue = 42;";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(&symbols, &["AutoValue"]);
    }

    #[test]
    fn can_parse_operator_overloads() {
        let code = "auto operator+(int a, int b) -> int;
                         auto operator new(size_t size) -> void*;
                         auto operator new[](size_t size) -> void*;
                         auto operator delete(void* ptr) -> void;
                         auto operator delete[](void* ptr) -> void;
                         auto operator co_await(Task<void>) -> Awaiter<void>;
                         auto operator[](int index) -> int&;
                         auto operator()(int a, int b) -> int;";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(&symbols, &["operator+", "operator new", "operator new[]", "operator delete", "operator delete[]", "operator co_await", "operator[]", "operator()"]);
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
        let Symbol {
            name,
            kind: SymbolKind::Namespace(namespace),
            ..
        } = &symbols[0]
        else {
            assert!(false, "Expected a namespace symbol");
            unreachable!()
        };
        assert_eq!(*name, "V1");
        assert_eq!(namespace.symbols.len(), 1);
        assert_eq!(&namespace.symbols[0].name, "VersionedType");
        assert!(namespace.is_inline);
        assert_matches!(&namespace.symbols[0].kind, SymbolKind::ExportableSymbol);
    }

    #[test]
    fn can_parse_requires_clauses() {
        let code = "template<typename T>
                          requires requires(T t) { t.foo(); }
                          void foo(T t) { t.foo(); }

                          template<typename T>
                          void bar() noexcept requires std::is_same_v<T, int>;

                          template<typename... T>
                            requires requires(T t) { t.foo(); } && std::conjunction_v<std::is_same<T, int>...>
                          void baz() noexcept requires (std::is_convertible_to<T, int> && ...);";
        let tokens = lex(code);
        let guarded = to_guarded_tokens(&tokens);
        let result = parse_symbols(&guarded);
        assert!(result.is_ok());
        let symbols = result.unwrap();
        assert_declarations(&symbols, &["foo", "bar", "baz"]);
    }
}
