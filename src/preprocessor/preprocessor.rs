use crate::preprocessor::tokens::{IncludeDirective, IncludeTarget, MacroCandidate, MacroDefinition, PreprocessorToken, PreprocessorTokenKind};
use crate::preprocessor::{Lexeme, lex, tokens};
use std::collections::{HashMap, HashSet};
use itertools::Itertools;
use std::{fs};
use std::path::PathBuf;

pub struct PreprocessorOutput {
    pub source: String,
}

#[derive(Default)]
struct PreprocessorState {
    tokens: Vec<PreprocessorToken>,
    included: HashSet<PathBuf>,
    definitions: HashMap<String, MacroDefinition>,
}

pub fn preprocess(source: &str, include_paths: &[PathBuf]) -> anyhow::Result<PreprocessorOutput> {
    let mut result = PreprocessorState::default();
    let lexemes = lex(source);

    tokenize_target(lexemes, &mut result, include_paths, None);

    let mut source = String::new();

    for token in result.tokens {
        source.push_str(&token.as_source());
    }

    Ok(PreprocessorOutput { source })
}

fn tokenize_target(
    lexemes: Vec<Lexeme>,
    result: &mut PreprocessorState,
    include_paths: &[PathBuf],
    header_name: Option<PathBuf>,
) {
    let tokens = tokens::parse_tokens(lexemes);

    for token in tokens {
        match token.kind {
            PreprocessorTokenKind::Include(_) => {
                try_expand_include(result, token, include_paths, &header_name);
            }
            PreprocessorTokenKind::Define(definition) => {
                result
                    .definitions
                    .insert(definition.name.clone(), definition.definition.clone());
            }
            PreprocessorTokenKind::Undef(definition) => {
                result.definitions.remove(&definition.name);
            }
            PreprocessorTokenKind::PragmaOnce if let Some(header) = header_name.as_ref() => {
                result.included.insert(header.clone());
            }
            PreprocessorTokenKind::MacroCandidate(_) => {
                try_expand_macro(result, token, include_paths, header_name.as_ref())
            }
            _ => result.tokens.push(token),
        }
    }
}

fn try_expand_include(
    result: &mut PreprocessorState,
    token: PreprocessorToken,
    include_paths: &[PathBuf],
    header_name: &Option<PathBuf>,
) {
    let PreprocessorToken { kind, original } = token;

    let include = match kind {
        PreprocessorTokenKind::Include(include) => include,
        kind => {
            result.tokens.push(PreprocessorToken { kind, original });
            return;
        }
    };

    match include.target {
        IncludeTarget::Angled(name) => {
            if let Some(header) = try_find_header(&name, include_paths, None) {
                tokenize_header(result, include_paths, header);
                return;
            }

            result.tokens.push(PreprocessorToken {
                kind: PreprocessorTokenKind::Include(IncludeDirective {
                    target: IncludeTarget::Angled(name),
                }),
                original,
            });
        }

        IncludeTarget::Quoted(name) => {
            if let Some(header) = try_find_header(&name, include_paths, header_name.as_ref()) {
                tokenize_header(result, include_paths, header);
                return;
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
}

fn tokenize_header(result: &mut PreprocessorState, include_paths: &[PathBuf], header: PathBuf) {
    if result.included.contains(&header) {
        return;
    }

    let contents = fs::read_to_string(&header).expect("Unable to read file");
    let lexemes = lex(&contents);

    tokenize_target(lexemes, result, include_paths, Some(header));
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
    include_paths: &[PathBuf],
    header_name: Option<&PathBuf>,
) {
    let PreprocessorTokenKind::MacroCandidate(ref candidate) = token.kind else {
        result.tokens.push(token);
        return;
    };

    let Some(definition) = result.definitions.get(&candidate.name) else {
        result.tokens.push(token);
        return;
    };

    let Some(lexemes) = expand_macro(&token.original, candidate, definition) else {
        result.tokens.push(token);
        return;
    };
    tokenize_target(lexemes, result, include_paths, header_name.cloned());
}

fn expand_macro(original: &[Lexeme], candidate: &MacroCandidate, definition: &MacroDefinition) -> Option<Vec<Lexeme>> {
    match definition {
        MacroDefinition::ObjectLike {
            replacement
        } => {
            let mut result = Vec::new();
            result.reserve(replacement.len() + original.len() - 1);
            result.extend(replacement.iter().cloned());
            result.extend(original.iter().skip(1).cloned());
            Some(result)
        }
        MacroDefinition::FunctionLike {
            parameters, variadic, replacement
        } => {
            let Some(provided_params) = candidate.parameters.as_ref() else {
                return None
            };
            
            if provided_params.len() < parameters.len() {
                return None
            }
            
            if provided_params.len() > parameters.len() && !variadic {
                return None;
            }

            let variadic_pack = &provided_params[parameters.len()..];
            let param_lookup: HashMap<&str, &Vec<Lexeme>> = parameters.iter()
                .zip(provided_params[..parameters.len()].iter())
                .map(|(param, arg)| (param.as_str(), arg))
                .collect();

            let mut result = Vec::new();
            for lexeme in replacement {
                match lexeme {
                    Lexeme::Identifier(name) => {
                        if let Some(arg) = param_lookup.get(name.as_str()) {
                            result.extend(arg.iter().cloned())
                        }
                        else if name == "__VA_ARGS__" {
                            let comma = vec![Lexeme::Comma];
                            result.extend(variadic_pack.iter()
                                .intersperse(&comma)
                                .flatten()
                                .cloned())
                        }
                        else {
                            result.push(lexeme.clone())
                        }
                    }
                    _ => result.push(lexeme.clone())
                }
            }
            Some(result)
        }
        MacroDefinition::Malformed {
            tokens: _
        } => None
    }
}