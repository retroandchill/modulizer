use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::fmt::Formatter;
use std::path::PathBuf;
use logos::Logos;
use crate::config::Config;
use crate::parser::grammar::Token;
use crate::parser::macros::{parse_expandable_syntax, ExpandableSyntax, MacroExpansionCandidate};
use crate::parser::preprocessor::{collect_statements, parse_include_expansion, ConditionalDirective, DefineDirective, DirectiveStatement, IncludeDirective, IncludePath, MacroParameters, PreprocessorStatement};

pub struct TranslationUnit {
    tokens: Vec<Token>,
}

struct TranslationUnitState<'a> {
    config: &'a Config,
    tokens: Vec<Token>,
    definitions: HashMap<String, DefineDirective>,
    header_stack: VecDeque<PathBuf>,
    seen_headers: HashSet<PathBuf>
}

impl TranslationUnit {
    pub fn new(config: &Config, source: &str) -> anyhow::Result<Self> {
        let mut state = TranslationUnitState {
            config,
            tokens: Vec::new(),
            definitions: get_initial_macro_definitions(config),
            header_stack: VecDeque::new(),
            seen_headers: HashSet::new()
        };

        state.parse_content(source)?;


        Ok(Self {
            tokens: state.tokens,
        })
    }
}

fn get_initial_macro_definitions(config: &Config) -> HashMap<String, DefineDirective> {
    let mut definitions = HashMap::new();
    for directive in &config.macros.explicit_macros {
        let Some((name, replacement)) = directive.split_once("=") else {
            definitions.insert(directive.clone(), DefineDirective {
                name: directive.clone(),
                parameters: None,
                replacement: Vec::new(),
            });
            continue;
        };

        let tokens = lex(replacement);
        definitions.insert(name.to_string(), DefineDirective {
            name: name.to_string(),
            parameters: None,
            replacement: tokens,
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

impl fmt::Display for TranslationUnit {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        for token in &self.tokens {
            write!(f, "{}", token)?;
        }
        Ok(())
    }
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

                    self.tokens.append(&mut tokens);
                    self.tokens.push(Token::NewLine);
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
                self.parse_definition(define);
            }
            DirectiveStatement::Undef(undef) => {
                self.parse_undefine(undef.name);
            }
            DirectiveStatement::Conditional(conditional) => {
                self.parse_conditional(conditional);
            }
            DirectiveStatement::Else => {
                self.tokens.push(Token::Hash);
                self.tokens.push(Token::Identifier("else".to_string()));
                self.tokens.push(Token::NewLine);
            }
            DirectiveStatement::Endif => {
                self.tokens.push(Token::Hash);
                self.tokens.push(Token::Identifier("endif".to_string()));
                self.tokens.push(Token::NewLine);
            }
            DirectiveStatement::Other(other) => {
                self.tokens.push(Token::Hash);
                self.tokens.push(Token::Identifier(other.name.to_string()));
                self.tokens.push(Token::Whitespace(" ".to_string()));
                self.tokens.extend_from_slice(&other.expression);
                self.tokens.push(Token::NewLine);
            }
        }

        Ok(())
    }

    fn parse_include(&mut self, directive: IncludeDirective) -> anyhow::Result<()> {
        match directive.path {
            IncludePath::System(path) => self.try_expand_header(&directive.tokens, path, false),
            IncludePath::Local(path) => self.try_expand_header(&directive.tokens, path, true),
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

    fn try_expand_header(&mut self, source_tokens: &[Token], path: PathBuf, search_current_path: bool) -> anyhow::Result<()> {
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
        if self.config.macros.expand_from_definition.contains(&define.name) {
            self.definitions.insert(define.name.clone(), define);
        }
    }

    fn parse_undefine(&mut self, name: String) {
        if self.config.macros.expand_from_definition.contains(&name) {
            self.definitions.remove(&name);
        }
    }

    fn parse_conditional(&mut self, conditional: ConditionalDirective) {
        self.tokens.push(Token::Hash);
        match conditional {
            ConditionalDirective::If { expression } => {
                self.tokens.push(Token::Identifier("if".to_string()));
                self.tokens.push(Token::Whitespace(" ".to_string()));
                self.tokens.extend_from_slice(&expression);
            }
            ConditionalDirective::Ifdef { name } => {
                self.tokens.push(Token::Identifier("ifdef".to_string()));
                self.tokens.push(Token::Whitespace(" ".to_string()));
                self.tokens.push(Token::Identifier(name));
            }
            ConditionalDirective::Ifndef { name } => {
                self.tokens.push(Token::Identifier("ifndef".to_string()));
                self.tokens.push(Token::Whitespace(" ".to_string()));
                self.tokens.push(Token::Identifier(name));
            }
            ConditionalDirective::Elif { expression } => {
                self.tokens.push(Token::Identifier("elif".to_string()));
                self.tokens.push(Token::Whitespace(" ".to_string()));
                self.tokens.extend_from_slice(&expression);
            }
            ConditionalDirective::Elifdef { name } => {
                self.tokens.push(Token::Identifier("elifdef".to_string()));
                self.tokens.push(Token::Whitespace(" ".to_string()));
                self.tokens.push(Token::Identifier(name));
            }
            ConditionalDirective::Elifndef { name } => {
                self.tokens.push(Token::Identifier("elifndef".to_string()));
                self.tokens.push(Token::Whitespace(" ".to_string()));
                self.tokens.push(Token::Identifier(name));
            }
        }
        self.tokens.push(Token::NewLine);
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
                expand_functional_macro(candidate, &definition.name, parameters, definition.replacement.as_slice(), tokens)?;
                Ok(true)
            }
            None => {
                tokens.extend_from_slice(definition.replacement.as_slice());
                if let Some(mut parameters) = candidate.parameters {
                    append_macro_parameters(&mut parameters, tokens);
                }
                Ok(true)
            }
        }

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
                            tokens.push(Token::Whitespace(" ".to_string()));
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
            tokens.push(Token::Whitespace(" ".to_string()));
        }

        tokens.append(&mut parameter);

        index += 1;
    }
    tokens.push(Token::RParen);
}