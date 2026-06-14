use crate::parser::grammar::tokens::{GuardedToken, PreprocessorGuard, Token};
use crate::parser::structure::{Delimiter, TokenGroup, TokenNode, collect_token_nodes};
use arraystring::ArrayString;
use arraystring::typenum::U25;
use chumsky::Parser;
use itertools::Itertools;
use std::fmt;
use std::fmt::Write;
use std::rc::Rc;
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
    ExternBlock(Vec<Symbol>),
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
            } else if !self.is_at_end() {
                self.skip_until_semicolon();
            }
        }

        symbols
    }

    fn parse_symbol(&mut self) -> Option<Symbol> {
        match self.try_peak_token()?.token {
            Token::Extern => self.parse_extern_block(),
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

    fn parse_extern_block(&mut self) -> Option<Symbol> {
        let declaration = self.expect_token(Token::Extern)?;
        if matches!(
            self.try_peak_token().map(|guarded| guarded.token),
            Some(Token::StringLiteral(_))
        ) {
            self.advance();
        }

        if let Some(group) = self.check_group(Delimiter::Braces) {
            self.advance();
            let sub_parser = SymbolParser::new(&group.children);
            let symbols = sub_parser.parse();
            return Some(Symbol {
                name: Ustr::default(),
                guards: declaration.guards,
                kind: SymbolKind::ExternBlock(symbols),
            });
        }

        self.parse_symbol()
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
        let name = self.parse_class_or_struct_declaration()?;
        if self.expect_token(Token::Semicolon).is_none() {
            return self.parse_inline_aggregate_symbol();
        }

        name.map(|name| Symbol {
            name: name.clone(),
            guards: declaration.guards,
            kind: SymbolKind::ExportableSymbol,
        })
    }

    fn parse_class_or_struct_declaration(&mut self) -> Option<Option<Ustr>> {
        let is_specialization;
        let name = match self.try_peak_token()?.token {
            Token::Identifier(name) => {
                self.advance();
                if self.check_token(Token::DoubleColon).is_some()
                    || self.check_token(Token::Less).is_some()
                {
                    is_specialization = true;
                } else {
                    is_specialization = false;
                }

                // Skip over the base class list if present
                self.skip_optional_base_class_list()?;
                Some(name)
            }
            _ => {
                is_specialization = false;
                None
            }
        };

        self.skip_optional_scope();
        Some(name.filter(|_| !is_specialization))
    }

    fn skip_optional_base_class_list(&mut self) -> Option<()> {
        self.expect_token(Token::Final);
        if self.expect_token(Token::Colon).is_none() {
            return Some(());
        };

        loop {
            let mut seen_virtual = false;
            let mut seen_access_specifier = false;
            loop {
                match self.try_peak_token().map(|token| token.token) {
                    Some(Token::Virtual) if !seen_virtual => {
                        self.advance();
                        seen_virtual = true;
                    }
                    Some(Token::Public | Token::Private | Token::Protected)
                        if !seen_access_specifier =>
                    {
                        self.advance();
                        seen_access_specifier = true;
                    }
                    _ => break,
                }
            }

            self.parse_templatable_identifier()?;

            if self.check_token(Token::Comma).is_some() {
                self.advance();
            } else {
                break;
            }
        }

        Some(())
    }

    fn parse_union(&mut self) -> Option<Symbol> {
        self.skip_declaration_qualifiers();
        let declaration = self.expect_token(Token::Union)?;
        let name = self.parse_union_declaration()?;
        if self.expect_token(Token::Semicolon).is_none() {
            return self.parse_inline_aggregate_symbol();
        }

        name.map(|name| Symbol {
            name: name.clone(),
            guards: declaration.guards,
            kind: SymbolKind::ExportableSymbol,
        })
    }

    fn parse_union_declaration(&mut self) -> Option<Option<Ustr>> {
        let name = match self.try_peak_token()?.token {
            Token::Identifier(name) => {
                self.advance();
                Some(name)
            }
            _ => None,
        };

        self.skip_optional_scope();
        Some(name)
    }

    fn parse_enum(&mut self) -> Option<Symbol> {
        self.skip_declaration_qualifiers();
        let declaration = self.expect_token(Token::Enum)?;

        if self.check_token(Token::Class).is_some() || self.check_token(Token::Struct).is_some() {
            self.advance();
        }
        let name = self.parse_enum_declaration()?;

        if self.expect_token(Token::Semicolon).is_none() {
            return self.parse_inline_aggregate_symbol();
        }

        name.map(|name| Symbol {
            name,
            guards: declaration.guards,
            kind: SymbolKind::ExportableSymbol,
        })
    }

    fn parse_enum_declaration(&mut self) -> Option<Option<Ustr>> {
        let name = match self.try_peak_token()?.token {
            Token::Identifier(name) => {
                self.advance();
                if self.expect_token(Token::Colon).is_some() {
                    self.parse_enum_base_type()?;
                };
                Some(name)
            }
            _ => None,
        };

        self.skip_optional_scope();
        Some(name)
    }

    fn parse_enum_base_type(&mut self) -> Option<()> {
        match self.try_peak_token()?.token {
            Token::Identifier(_) | Token::DoubleColon | Token::Decltype => {
                self.parse_templatable_identifier()?;
            }
            Token::Unsigned | Token::Signed => {
                self.advance();
                self.parse_sign_modified_type();
            }
            Token::Short | Token::LongLong => {
                self.advance();
                self.expect_token(Token::Int);
            }
            Token::Long => {
                self.advance();
                if self.expect_token(Token::Int).is_some()
                    || self.expect_token(Token::Double).is_some()
                {
                    return Some(());
                }
            }
            Token::Int
            | Token::Char
            | Token::WChar
            | Token::Char8
            | Token::Char16
            | Token::Char32 => {
                self.advance();
            }
            _ => return None,
        }

        Some(())
    }

    fn parse_inline_aggregate_symbol(&mut self) -> Option<Symbol> {
        self.skip_cv_ref_qualifiers();
        self.parse_variable_or_function_after_type(false)
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
            if self.is_at_end() {
                return None;
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
                TokenNode::Token(token) => match token.token {
                    Token::Identifier(_) | Token::DoubleColon | Token::Decltype => {
                        self.parse_templatable_identifier()?;
                    }
                    Token::Requires => {
                        self.advance();
                        self.expect_group(Delimiter::Parentheses);
                        self.expect_group(Delimiter::Braces)?;
                    }
                    _ => return None,
                },
                TokenNode::Group(group) => {
                    if group.delimiter == Delimiter::Parentheses {
                        self.advance();
                    } else {
                        return None;
                    }
                }
            }

            match self.try_peak_token().map(|token| token.token) {
                Some(Token::And | Token::Or) => self.advance(),
                _ => break,
            }
        }

        Some(())
    }

    fn parse_variable_or_function(&mut self) -> Option<Symbol> {
        self.skip_declaration_qualifiers();
        let is_auto = self.parse_type_specifier()?;
        self.parse_variable_or_function_after_type(is_auto)
    }

    fn parse_variable_or_function_after_type(&mut self, is_auto: bool) -> Option<Symbol> {
        let (name, guards, is_function) = self.parse_variable_or_function_name(is_auto)?;

        if is_function {
            self.parse_function_body(is_auto)?;
        } else if self.check_token(Token::Semicolon).is_some() {
            self.advance();
        } else if self.check_token(Token::Equal).is_some() {
            self.skip_until_semicolon();
        } else {
            return None;
        }

        Some(Symbol {
            name,
            guards,
            kind: SymbolKind::ExportableSymbol,
        })
    }

    fn parse_variable_or_function_name(
        &mut self,
        is_auto: bool,
    ) -> Option<(Ustr, Rc<[PreprocessorGuard]>, bool)> {
        let node = self.peek()?;
        match node {
            TokenNode::Token(identifier) => match identifier.token {
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
                    } else {
                        Some((name, identifier.guards, false))
                    }
                }
                Token::Operator => {
                    let name = self.parse_operator_overload_name()?;
                    self.expect_group(Delimiter::Parentheses)?;
                    Some((name, identifier.guards, true))
                }
                _ => None,
            },
            TokenNode::Group(group) => match group.delimiter {
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
                _ => return None,
            },
        }
    }

    fn parse_type_specifier(&mut self) -> Option<bool> {
        self.skip_optional_cv_qualifiers();
        let is_auto;
        if let Some(auto) = self.parse_fundamental_type() {
            is_auto = auto;
        } else {
            match self.try_peak_token().map(|token| token.token) {
                Some(Token::Typename) => {
                    self.advance();
                    self.parse_templatable_identifier()?;
                }
                Some(Token::Class | Token::Struct) => {
                    self.advance();
                    self.parse_class_or_struct_declaration()?;
                }
                Some(Token::Union) => {
                    self.advance();
                    self.parse_union_declaration()?;
                }
                Some(Token::Enum) => {
                    self.advance();
                    if self.check_token(Token::Class).is_some()
                        || self.check_token(Token::Struct).is_some()
                    {
                        self.advance();
                    }
                    self.parse_enum_declaration()?;
                }
                _ => {
                    self.parse_templatable_identifier()?;
                }
            }
            is_auto = false;
        }
        self.skip_cv_ref_qualifiers();

        let mut pointer_to_member_check = self.clone();
        if let Some(_) = pointer_to_member_check.parse_templatable_identifier() {
            if pointer_to_member_check
                .expect_token(Token::DoubleColon)
                .is_some()
                && pointer_to_member_check.check_token(Token::Star).is_some()
            {
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

            if !self.check_token(Token::DoubleColon).is_some()
                || !self.check_next_token(Token::Star).is_none()
            {
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
            Some(Token::Short | Token::LongLong) => {
                self.advance();
                self.expect_token(Token::Int);
                Some(false)
            }
            Some(Token::Long) => {
                self.advance();
                if !self.expect_token(Token::Int).is_some() {
                    self.expect_token(Token::Double);
                }
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
            TokenNode::Token(guarded) => match guarded.token {
                Token::New => {
                    self.advance();
                    if self
                        .check_group(Delimiter::Brackets)
                        .is_some_and(|group| group.children.is_empty())
                    {
                        self.advance();
                        return Some(Ustr::from("operator new[]"));
                    }

                    Some(Ustr::from("operator new"))
                }
                Token::Delete => {
                    self.advance();
                    if self
                        .check_group(Delimiter::Brackets)
                        .is_some_and(|group| group.children.is_empty())
                    {
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
                                } else {
                                    return None;
                                }
                            }
                        }
                    }

                    Some(Ustr::from(buffer.as_str()))
                }
            },
            TokenNode::Group(group) => {
                if !group.children.is_empty() {
                    return None;
                }

                match group.delimiter {
                    Delimiter::Parentheses => {
                        self.advance();
                        Some(Ustr::from("operator()"))
                    }
                    Delimiter::Brackets => {
                        self.advance();
                        Some(Ustr::from("operator[]"))
                    }
                    Delimiter::Braces => None,
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
        if self.is_at_end() {
            panic!("Tried to advance past the end of the input");
        }
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

pub fn parse_symbols(input: &[GuardedToken]) -> Result<Vec<Symbol>, SymbolError> {
    eprintln!("parse_symbols: starting");

    let nodes = collect_token_nodes(input);
    let result = SymbolParser::new(&nodes).parse();

    eprintln!("parse_symbols: finished");

    Ok(result)
}
