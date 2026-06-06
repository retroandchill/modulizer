use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt;
use std::fmt::Formatter;
use std::path::PathBuf;
use logos::Logos;
use crate::config::Config;
use crate::parser::grammar::Token;
use crate::parser::preprocessor::{collect_statements, ConditionalDirective, DefineDirective, DirectiveStatement, IncludeDirective, IncludePath, PreprocessorStatement};

pub struct TranslationUnit {
    tokens: Vec<Token>,
}

struct TranslationUnitState<'a> {
    config: &'a Config,
    tokens: Vec<Token>,
    definitions: HashMap<String, Vec<Token>>,
    header_stack: VecDeque<PathBuf>,
    seen_headers: HashSet<PathBuf>
}

impl TranslationUnit {
    pub fn new(config: &Config, source: &str) -> anyhow::Result<Self> {
        let mut state = TranslationUnitState {
            config,
            tokens: Vec::new(),
            definitions: HashMap::new(),
            header_stack: VecDeque::new(),
            seen_headers: HashSet::new()
        };

        state.parse_content(source)?;


        Ok(Self {
            tokens: state.tokens,
        })
    }
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
    fn lex(&mut self, source: &str) -> Vec<Token> {
        Token::lexer(source)
            .filter_map(|result| {
                result.ok()
            })
            .collect()
    }

    fn parse_content(&mut self, source: &str) -> anyhow::Result<()> {
        let lexemes = self.lex(source);
        let statements = collect_statements(&lexemes)
            .map_err(|err| anyhow::anyhow!("Failed to parse statements: {}", err))?;

        for statement in statements {
            match statement {
                PreprocessorStatement::Directive(directive) => self.parse_directive(directive)?,
                PreprocessorStatement::Code(code) => {
                    self.tokens.extend_from_slice(&code.tokens);
                }
            }

            self.tokens.push(Token::NewLine);
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
            }
            DirectiveStatement::Endif => {
                self.tokens.push(Token::Hash);
                self.tokens.push(Token::Identifier("endif".to_string()));
            }
            DirectiveStatement::Other(other) => {
                self.tokens.push(Token::Hash);
                self.tokens.push(Token::Identifier(other.name.to_string()));
                self.tokens.push(Token::Whitespace(" ".to_string()));
                self.tokens.extend_from_slice(&other.expression);
            }
        }

        Ok(())
    }

    fn parse_include(&mut self, directive: IncludeDirective) -> anyhow::Result<()> {
        match directive.path {
            IncludePath::System(path) => self.try_expand_header(&directive.tokens, path, false)?,
            IncludePath::Local(path) => self.try_expand_header(&directive.tokens, path, true)?,
            IncludePath::Macro => {

            },
        };

        Ok(())
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
            self.tokens.push(Token::Hash);
            self.tokens.push(Token::Identifier("include".to_string()));
            self.tokens.push(Token::Whitespace(" ".to_string()));
            if search_current_path {
                self.tokens.extend_from_slice(source_tokens);
            }
            else {
                self.tokens.push(Token::Less);
                self.tokens.extend_from_slice(source_tokens);
                self.tokens.push(Token::Greater);
            }

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
        self.tokens.push(Token::Hash);
        self.tokens.push(Token::Identifier("define".to_string()));
        self.tokens.push(Token::Whitespace(" ".to_string()));
        self.tokens.push(Token::Identifier(define.name));
        if let Some(parameters) = &define.parameters {
            self.tokens.push(Token::LParen);
            let mut index: usize = 0;
            for name in &parameters.names {
                if index > 0 {
                    self.tokens.push(Token::Comma);
                    self.tokens.push(Token::Whitespace(" ".to_string()));
                }
                self.tokens.push(Token::Identifier(name.to_string()));
                index += 1;
            }

            if parameters.variadic {
                if index > 0 {
                    self.tokens.push(Token::Comma);
                    self.tokens.push(Token::Whitespace(" ".to_string()));
                }

                self.tokens.push(Token::Ellipsis);
            }

            self.tokens.push(Token::RParen);
        }
        self.tokens.push(Token::Whitespace(" ".to_string()));
        self.tokens.extend_from_slice(define.replacement.as_slice());
    }

    fn parse_undefine(&mut self, name: String) {
        self.tokens.push(Token::Hash);
        self.tokens.push(Token::Identifier("undef".to_string()));
        self.tokens.push(Token::Whitespace(" ".to_string()));
        self.tokens.push(Token::Identifier(name));
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
    }
}