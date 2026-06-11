use std::collections::{HashMap, HashSet, VecDeque};
use std::collections::hash_map::Entry;
use std::path::PathBuf;
use std::rc::Rc;
use itertools::Itertools;
use logos::Logos;
use ustr::{Ustr, UstrMap};
use crate::config::Config;
use crate::parser::grammar::{GuardedTokens, PreprocessorGuard, Token};
use crate::parser::macros::{parse_expandable_syntax, ExpandableSyntax, MacroExpansionCandidate};
use crate::parser::preprocessor::{collect_statements, parse_include_expansion, ConditionalDirective, DefineDirective, DirectiveStatement, IncludeDirective, IncludePath, MacroParameters, PreprocessorStatement};
use crate::parser::symbols::{parse_symbols, Namespace, Symbol, SymbolKind};

pub struct TranslationUnit {
    symbols: Vec<Symbol>,
    macros: HashSet<Ustr>,
}

struct TranslationUnitState<'a> {
    config: &'a Config,
    tokens: Vec<GuardedTokens>,
    definitions: HashMap<Ustr, DefineDirective>,
    header_stack: VecDeque<PathBuf>,
    seen_headers: HashSet<PathBuf>,
    all_macros: HashSet<Ustr>,
    guards: VecDeque<PreprocessorGuard>,
}

impl TranslationUnit {
    pub fn new(config: &Config, source: &str) -> anyhow::Result<Self> {
        let mut state = TranslationUnitState {
            config,
            tokens: Vec::new(),
            definitions: get_initial_macro_definitions(config),
            header_stack: VecDeque::new(),
            seen_headers: HashSet::new(),
            all_macros: HashSet::new(),
            guards: VecDeque::new(),
        };

        state.parse_content(source)?;


        Ok(Self {
            symbols: state.collect_symbols()?,
            macros: state.all_macros,
        })
    }

    pub fn has_macros(&self) -> bool {
        !self.macros.is_empty()
    }

    pub fn macros(&self) -> impl Iterator<Item = &str> {
        self.macros.iter()
            .map(|macro_name| macro_name.as_str())
            .sorted()
    }

    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.iter()
    }
}

fn get_initial_macro_definitions(config: &Config) -> HashMap<Ustr, DefineDirective> {
    let mut definitions = HashMap::new();
    for directive in &config.macros.explicit_macros {
        let directive = Ustr::from(directive);
        let Some((name, replacement)) = directive.split_once("=") else {
            definitions.insert(directive.clone(), DefineDirective {
                name: directive,
                parameters: None,
                replacement: Rc::new([]),
            });
            continue;
        };

        let tokens = lex(replacement);
        definitions.insert(Ustr::from(name), DefineDirective {
            name: directive,
            parameters: None,
            replacement: Rc::from(tokens),
        });
    }
    definitions
}

fn lex(source: &str) -> Vec<Token> {
    Token::lexer(source)
        .filter_map(|result| {
            result.ok()
        })
        .collect()
}

impl<'a> TranslationUnitState<'a> {
    fn parse_content(&mut self, source: &str) -> anyhow::Result<()> {
        let lexemes = lex(source);
        let statements = collect_statements(&lexemes)
            .map_err(|err| anyhow::anyhow!("Failed to parse statements: {}", err))?;

        for statement in statements {
            match statement {
                PreprocessorStatement::Directive(directive) => self.parse_directive(directive)?,
                PreprocessorStatement::Code(code) => {
                    let mut tokens = code.tokens;
                    let mut expanded = true;
                    while expanded {
                        let syntax = parse_expandable_syntax(&tokens)
                            .map_err(|err| anyhow::anyhow!("Failed to expand macro: {}", err))?;

                        (tokens, expanded) = self.expand_macros(syntax)?;
                    }

                    let mut guarded_tokens = GuardedTokens::new(self.guards.iter()
                        .filter(|guard| {
                            match guard {
                                PreprocessorGuard::Conditional(ConditionalDirective::Ifndef { name }) => {
                                    self.config.headers.header_guard_format.as_ref().map(|r| {
                                        !r.is_match(name)
                                    })
                                        .unwrap_or(true)
                                }
                                _ => true,
                            }
                        })
                        .cloned());
                    guarded_tokens.append(tokens.into_iter());
                    self.tokens.push(guarded_tokens);
                }
            }
        }


        self.header_stack.pop_back();

        Ok(())
    }

    fn parse_directive(&mut self, directive: DirectiveStatement) -> anyhow::Result<()> {
        match directive {
            DirectiveStatement::Include(include) => {
                self.parse_include(include)?;
            }
            DirectiveStatement::Define(define) => {
                self.all_macros.insert(define.name.clone());
                self.parse_definition(define);
            }
            DirectiveStatement::Undef(undef) => {
                self.all_macros.remove(&undef.name);
                self.parse_undefine(undef.name);
            }
            DirectiveStatement::Conditional(conditional) => {
                self.parse_conditional(conditional)?;
            }
            DirectiveStatement::Else => {
                let Some(guard) = self.guards.back_mut() else {
                    return Err(anyhow::anyhow!("Else without preceding if"));
                };
                *guard = PreprocessorGuard::Else;
            }
            DirectiveStatement::Endif => {
                if self.guards.pop_back().is_none() {
                    return Err(anyhow::anyhow!("Endif without preceding if"));
                }
            }
            DirectiveStatement::Other => {
                // We want to just discard these, since they don't affect the analysis
            }
        }

        Ok(())
    }

    fn parse_include(&mut self, directive: IncludeDirective) -> anyhow::Result<()> {
        match directive.path {
            IncludePath::System(path) => self.try_expand_header(path, false),
            IncludePath::Local(path) => self.try_expand_header(path, true),
            IncludePath::Macro => {
                let syntax = parse_expandable_syntax(&directive.tokens)
                    .map_err(|err| anyhow::anyhow!("Failed to expand macro: {}", err))?;

                let (tokens, expanded) = self.expand_macros(syntax)?;
                if !expanded {
                    return Err(anyhow::anyhow!("Failed to expand macro into a valid include"));
                }

                let expansion = parse_include_expansion(&tokens)
                    .map_err(|err| anyhow::anyhow!("Failed to parse include expansion: {}", err))?;

                self.parse_include(expansion)
            },
        }
    }

    fn try_expand_header(&mut self, path: PathBuf, search_current_path: bool) -> anyhow::Result<()> {
        let mut target = None;
        if let Some(parent) = self.header_stack.back().and_then(|h| h.parent()) && search_current_path {
            target = Some(parent.join(&path)).filter(|p| p.exists());
        }

        if target.is_none() {
            for include_path in &self.config.headers.include_dirs {
                target = Some(include_path.join(&path)).filter(|p| p.exists());
                if target.is_some() {
                    break;
                }
            }
        }

        let Some(header) = target else {
            return Ok(());
        };

        if self.seen_headers.contains(&header) {
            return Ok(());
        }

        let source = std::fs::read_to_string(&header)?;
        self.seen_headers.insert(header.clone());
        self.header_stack.push_back(header);

        self.parse_content(&source)?;

       Ok(())
    }

    fn parse_definition(&mut self, define: DefineDirective) {
        if self.config.macros.expand_from_definition.contains(define.name.as_str()) {
            self.definitions.insert(define.name.clone(), define);
        }
    }

    fn parse_undefine(&mut self, name: Ustr) {
        if self.config.macros.expand_from_definition.contains(name.as_str()) {
            self.definitions.remove(&name);
        }
    }

    fn parse_conditional(&mut self, conditional: ConditionalDirective) -> anyhow::Result<()> {
        match &conditional {
            ConditionalDirective::If { .. }| ConditionalDirective::Ifdef { .. } | ConditionalDirective::Ifndef { .. } => {
                self.guards.push_back(PreprocessorGuard::Conditional(conditional));
            }
            ConditionalDirective::Elif { .. } | ConditionalDirective::Elifdef { .. } | ConditionalDirective::Elifndef { .. } => {
                let Some(guard) = self.guards.back_mut() else {
                    return Err(anyhow::anyhow!("Elif without preceding if"));
                };
                *guard = PreprocessorGuard::Conditional(conditional);
            }
        }
        Ok(())
    }

    fn expand_macros(&mut self, syntax: Vec<ExpandableSyntax>) -> anyhow::Result<(Vec<Token>, bool)> {
        let mut tokens = Vec::new();
        let mut expanded = false;
        for expression in syntax {
            match expression {
                ExpandableSyntax::Candidate(candidate) => {
                    expanded |= self.try_expand_macro(candidate, &mut tokens)?;
                }
                ExpandableSyntax::Expression(mut expression) => {
                    tokens.append(&mut expression);
                }
            }
        }

        Ok((tokens, expanded))
    }

    fn try_expand_macro(&mut self, candidate: MacroExpansionCandidate, tokens: &mut Vec<Token>) -> anyhow::Result<bool> {
        let Some(definition) = self.definitions.get(&candidate.name) else {
            tokens.push(Token::Identifier(candidate.name));
            if let Some(mut parameters) = candidate.parameters {
                append_macro_parameters(&mut parameters, tokens);
            }
            return Ok(false);
        };

        match &definition.parameters {
            Some(parameters) => {
                expand_functional_macro(candidate, &definition.name, parameters, &definition.replacement, tokens)?;
                Ok(true)
            }
            None => {
                tokens.extend_from_slice(&definition.replacement);
                if let Some(mut parameters) = candidate.parameters {
                    append_macro_parameters(&mut parameters, tokens);
                }
                Ok(true)
            }
        }

    }

    fn collect_symbols(&self) -> anyhow::Result<Vec<Symbol>> {
        let tokens = self.tokens.iter()
            .flat_map(|guard| guard.into_iter())
            .collect::<Vec<_>>();

        let raw_symbols = parse_symbols(tokens.as_slice())
            .map_err(|err| anyhow::anyhow!("Failed to parse symbols: {}", err))?;

        let symbols = self.merge_symbol_sets(raw_symbols, "", false);

        Ok(symbols)
    }

    fn merge_symbol_sets(&self, raw_symbols: Vec<Symbol>, parent_scope: &str, parent_is_excluded: bool) -> Vec<Symbol> {
        let mut symbols = Vec::with_capacity(raw_symbols.len());
        let mut seen_symbols = HashMap::new();
        for symbol in raw_symbols {
            if let SymbolKind::Namespace(mut namespace) = symbol.kind {
                if namespace.is_empty() {
                    continue;
                }

                if let Some(Symbol { kind: SymbolKind::Namespace(existing_namespace), guards, .. }) = seen_symbols.get(&symbol.name)
                    .and_then(|index| -> Option<&mut Symbol> { symbols.get_mut(*index) }) {
                    existing_namespace.symbols.append(&mut namespace.symbols);
                    *guards = reduce_guard_set(guards.clone(), symbol.guards);
                } else {
                    seen_symbols.insert(symbol.name.clone(), symbols.len());
                    symbols.push(Symbol {
                        name: symbol.name,
                        guards: symbol.guards,
                        kind: SymbolKind::Namespace(namespace),
                    });
                }
            } else {
                if let Entry::Vacant(slot) = seen_symbols.entry(symbol.name.clone()) {
                    slot.insert(symbols.len());
                    symbols.push(symbol);
                }
            }
        }

        let mut final_symbols = Vec::with_capacity(symbols.len());
        for symbol in symbols {
            let current_scope = if parent_scope.is_empty() {
                symbol.name.clone()
            } else {
                Ustr::from(format!("{}::{}", parent_scope, symbol.name).as_str())
            };
            let is_excluded = parent_is_excluded || self.config.symbols.exclude.contains(current_scope.as_str());
            if is_excluded && (self.config.symbols.include.contains(current_scope.as_str()) || !self.config.symbols.include.iter().any(|include| include.starts_with(format!("{}::", current_scope).as_str()))) {
                continue;
            }

            if let SymbolKind::Namespace(namespace) = symbol.kind {
                let mut merged_symbols = self.merge_symbol_sets(namespace.symbols, &current_scope, is_excluded);
                let mut name = symbol.name;
                if merged_symbols.len() == 1 {
                    if let Some(Symbol { kind: SymbolKind::Namespace(sub_namespace), name: sub_name, guards }) = merged_symbols.pop_if(|symbol| {
                        matches!(symbol.kind, SymbolKind::Namespace(_))
                    }) {
                        if !sub_namespace.is_inline {
                            merged_symbols = sub_namespace.symbols;
                            name = Ustr::from(format!("{}::{}", name, sub_name).as_str());
                        }
                        else {
                            merged_symbols.push(Symbol {
                                name: sub_name,
                                guards,
                                kind: SymbolKind::Namespace(sub_namespace),
                            });
                        }
                    }
                }
                final_symbols.push(Symbol {
                    name,
                    guards: symbol.guards,
                    kind: SymbolKind::Namespace(Namespace {
                        is_inline: namespace.is_inline,
                        symbols: merged_symbols,
                    }),
                });
            } else {
                final_symbols.push(symbol);
            }
        }

        final_symbols
    }
}

fn reduce_guard_set(existing: Rc<[PreprocessorGuard]>, new: Rc<[PreprocessorGuard]>) -> Rc<[PreprocessorGuard]> {
    if existing.is_empty() {
        return existing;
    }

    let reduced_index = existing.iter().zip(new.iter())
        .enumerate()
        .filter(|(_, (existing, new))| {
            **existing != **new
        })
        .map(|(i, _)| i)
        .next();

    if let Some(index) = reduced_index {
        Rc::from(&existing[0..index])
    } else {
        Rc::new([])
    }
}

fn expand_functional_macro(candidate: MacroExpansionCandidate, name: &str, parameters: &MacroParameters, replacement: &[Token], tokens: &mut Vec<Token>) -> anyhow::Result<()> {
    let Some(provided_parameters) = candidate.parameters else {
        return Err(anyhow::anyhow!("Macro {} was used, but no parameters were provided", name));
    };

    if provided_parameters.len() > parameters.names.len() && !parameters.variadic {
        return Err(anyhow::anyhow!("Macro {} was used with too many parameters", name));
    }

    if provided_parameters.len() < parameters.names.len() {
        return Err(anyhow::anyhow!("Macro {} was used with too few parameters", name));
    }

    let lookup_name = |name: &str| -> Option<&[Token]> {
        for (i, parameter) in parameters.names.iter().enumerate() {
            if i >= provided_parameters.len() {
                return None;
            }

            if parameter == name {
                return Some(&provided_parameters[i]);
            }
        }

        None
    };

    let variadic_pack = &provided_parameters[parameters.names.len()..];

    for token in replacement {
        match token {
            Token::Identifier(identifier) => {
                if identifier == "__VA_ARGS__" {
                    let mut index: usize = 0;
                    for parameter_set in variadic_pack {
                        if index > 0 {
                            tokens.push(Token::Comma);
                            tokens.push(Token::Whitespace);
                        }

                        tokens.append(&mut parameter_set.clone());
                        index += 1;
                    }
                }
                else if let Some(parameter_set) = lookup_name(identifier) {
                    tokens.extend_from_slice(parameter_set);
                }
                else {
                    tokens.push(token.clone());
                }
            }
            _ => {
                tokens.push(token.clone());
            }
        }
    }


    Ok(())
}

fn append_macro_parameters(parameters: &mut Vec<Vec<Token>>, tokens: &mut Vec<Token>) {
    let mut index: usize = 0;
    tokens.push(Token::LParen);
    for mut parameter in parameters {
        if index > 0 {
            tokens.push(Token::Comma);
            tokens.push(Token::Whitespace);
        }

        tokens.append(&mut parameter);

        index += 1;
    }
    tokens.push(Token::RParen);
}