use crate::preprocessor::tokens::{
    IncludeDirective, IncludeTarget, MacroCandidate, MacroDefinition,
    PreprocessorToken, PreprocessorTokenKind, parse_tokens,
};
use crate::preprocessor::{Lexeme, lex};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::fmt::Formatter;
use std::fs;
use std::path::PathBuf;
use crate::config::Config;

pub struct PreprocessorOutput {
    pub source: String,
}

#[derive(Default)]
struct PreprocessorState {
    tokens: Vec<PreprocessorToken>,
    included: HashSet<PathBuf>,
    definitions: HashMap<String, MacroDefinition>,
    non_macros: HashSet<String>
}

#[derive(Debug, Clone)]
pub struct PreprocessError {
    pub message: String,
}

impl std::fmt::Display for PreprocessError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PreprocessError {}

pub fn preprocess(source: &str, config: &Config) -> anyhow::Result<PreprocessorOutput> {
    let mut result = PreprocessorState::default();
    let lexemes = lex(source);

    tokenize_target(lexemes, &mut result, &config, None)?;

    let mut source = String::new();

    for token in result.tokens {
        source.push_str(&token.as_source());
    }

    Ok(PreprocessorOutput { source })
}

fn tokenize_target(
    lexemes: Vec<Lexeme>,
    result: &mut PreprocessorState,
    config: &Config,
    header_name: Option<&PathBuf>,
) -> Result<(), PreprocessError> {
    let tokens = parse_tokens(lexemes, &result.non_macros);

    for token in tokens {
        match token.kind {
            PreprocessorTokenKind::Include(_) => {
                try_expand_include(result, token, config, header_name)?;
            }
            PreprocessorTokenKind::Define(ref definition) => {
                if config.macros.expand_from_definition.contains(&definition.name) {
                    result
                        .definitions
                        .insert(definition.name.clone(), definition.definition.clone());
                    result.non_macros.remove(&definition.name);
                }
                result.tokens.push(token)
            }
            PreprocessorTokenKind::Undef(ref definition) => {
                if config.macros.expand_from_definition.contains(&definition.name) {
                    result.definitions.remove(&definition.name);
                    result.non_macros.insert(definition.name.clone());
                }
                result.tokens.push(token)
            }
            PreprocessorTokenKind::If(_) => result.tokens.push(token),
            PreprocessorTokenKind::IfDef(_) => result.tokens.push(token),
            PreprocessorTokenKind::IfNDef(_) => result.tokens.push(token),
            PreprocessorTokenKind::Elif(_) => result.tokens.push(token),
            PreprocessorTokenKind::Elifdef(_) => result.tokens.push(token),
            PreprocessorTokenKind::Elifndef(_) =>
                result.tokens.push(token),
            PreprocessorTokenKind::Else => {
                result.tokens.push(token);
            },
            PreprocessorTokenKind::EndIf => {
                result.tokens.push(token);
            }
            PreprocessorTokenKind::MacroCandidate(_) => {
                try_expand_macro(result, token, config, header_name)?
            }
            PreprocessorTokenKind::Text | PreprocessorTokenKind::OtherDirective => {
                result.tokens.push(token);
            }
        }
    }

    Ok(())
}

fn try_expand_include(
    result: &mut PreprocessorState,
    token: PreprocessorToken,
    config: &Config,
    header_name: Option<&PathBuf>,
) -> Result<(), PreprocessError> {
    let PreprocessorToken { kind, original } = token;

    let include = match kind {
        PreprocessorTokenKind::Include(include) => include,
        kind => {
            result.tokens.push(PreprocessorToken { kind, original });
            return Ok(());
        }
    };

    match include.target {
        IncludeTarget::Angled(name) => {
            if let Some(ref header) = try_find_header(&name, &config.headers.include_dirs, None) {
                println!("Expanding angled header: {}", name);
                return tokenize_header(result, config, header);
            }

            result.tokens.push(PreprocessorToken {
                kind: PreprocessorTokenKind::Include(IncludeDirective {
                    target: IncludeTarget::Angled(name),
                }),
                original,
            });
        }

        IncludeTarget::Quoted(name) => {
            if let Some(ref header) = try_find_header(&name, &config.headers.include_dirs, header_name) {
                println!("Expanding quoted header: {}", name);
                return tokenize_header(result, config, header);
            }

            result.tokens.push(PreprocessorToken {
                kind: PreprocessorTokenKind::Include(IncludeDirective {
                    target: IncludeTarget::Quoted(name),
                }),
                original,
            });
        }

        IncludeTarget::Macro(lexemes) => {
            result.tokens.push(PreprocessorToken {
                kind: PreprocessorTokenKind::Include(IncludeDirective {
                    target: IncludeTarget::Macro(lexemes),
                }),
                original,
            });
        }
        IncludeTarget::Malformed(lexemes) => {
            result.tokens.push(PreprocessorToken {
                kind: PreprocessorTokenKind::Include(IncludeDirective {
                    target: IncludeTarget::Malformed(lexemes),
                }),
                original,
            });
        }
    }

    Ok(())
}

fn tokenize_header(
    result: &mut PreprocessorState,
    include_paths: &Config,
    header: &PathBuf,
) -> Result<(), PreprocessError> {
    if result.included.contains(header) {
        return Ok(());
    }

    result.included.insert(header.clone());
    let contents = fs::read_to_string(&header)
        .map_err(|e| PreprocessError {
            message: format!("Failed to read header file: {}", e),
        })?;
    let lexemes = lex(&contents);

    tokenize_target(lexemes, result, include_paths, Some(header))
}

fn try_find_header<'a>(
    name: &'a str,
    include_paths: &[PathBuf],
    header_name: Option<&PathBuf>,
) -> Option<PathBuf> {
    if let Some(path) = header_name.and_then(|h| h.parent()) {
        let target_path = path.join(name);
        if target_path.exists() {
            return Some(target_path);
        }
    }

    for path in include_paths {
        let target_path = path.join(name);

        if target_path.exists() {
            return Some(target_path);
        }
    }

    None
}

fn try_expand_macro(
    result: &mut PreprocessorState,
    token: PreprocessorToken,
    config: &Config,
    header_name: Option<&PathBuf>,
) -> Result<(), PreprocessError> {
    let PreprocessorTokenKind::MacroCandidate(ref candidate) = token.kind else {
        result.tokens.push(token);
        return Ok(());
    };

    let Some(definition) = result.definitions.get(&candidate.name) else {
        result.non_macros.insert(candidate.name.clone());
        return tokenize_target(token.original, result, config, header_name);
    };

    let Some(lexemes) = expand_macro(&token.original, candidate, definition) else {
        result.non_macros.insert(candidate.name.clone());
        return tokenize_target(token.original, result, config, header_name);
    };
    tokenize_target(lexemes, result, config, header_name)
}

fn expand_macro(
    original: &[Lexeme],
    candidate: &MacroCandidate,
    definition: &MacroDefinition,
) -> Option<Vec<Lexeme>> {
    match definition {
        MacroDefinition::ObjectLike { replacement } => {
            let mut result = Vec::new();
            result.reserve(replacement.len() + original.len() - 1);
            result.extend(replacement.iter().cloned());
            result.extend(original.iter().skip(1).cloned());
            Some(result)
        }
        MacroDefinition::FunctionLike {
            parameters,
            variadic,
            replacement,
        } => {
            let Some(provided_params) = candidate.parameters.as_ref() else {
                return None;
            };

            if provided_params.len() < parameters.len() {
                return None;
            }

            if provided_params.len() > parameters.len() && !variadic {
                return None;
            }

            let variadic_pack = &provided_params[parameters.len()..];
            let param_lookup: HashMap<&str, &Vec<Lexeme>> = parameters
                .iter()
                .zip(provided_params[..parameters.len()].iter())
                .map(|(param, arg)| (param.as_str(), arg))
                .collect();

            let mut result = Vec::new();
            for lexeme in replacement {
                match lexeme {
                    Lexeme::Identifier(name) => {
                        if let Some(arg) = param_lookup.get(name.as_str()) {
                            result.extend(arg.iter().cloned())
                        } else if name == "__VA_ARGS__" {
                            let comma = vec![Lexeme::Comma];
                            result
                                .extend(variadic_pack.iter().intersperse(&comma).flatten().cloned())
                        } else {
                            result.push(lexeme.clone())
                        }
                    }
                    _ => result.push(lexeme.clone()),
                }
            }
            Some(result)
        }
        MacroDefinition::Malformed { tokens: _ } => None,
    }
}
