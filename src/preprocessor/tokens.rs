use std::iter;
use crate::preprocessor::Lexeme;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreprocessorToken {
    pub kind: PreprocessorTokenKind,
    pub original: Vec<Lexeme>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreprocessorTokenKind {
    Include(IncludeDirective),
    Define(DefineDirective),
    Undef(NameDirective),

    If(ConditionalDirective),
    IfDef(NameDirective),
    IfNDef(NameDirective),
    Elif(ConditionalDirective),
    Elifdef(NameDirective),
    Elifndef(NameDirective),
    Else,
    EndIf,

    PragmaOnce,

    OtherDirective,
    MacroCandidate(MacroCandidate),
    Text,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameDirective {
    pub name: String
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IncludeDirective {
    pub target: IncludeTarget
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IncludeTarget {
    Quoted(String),
    Angled(String),
    Macro(Vec<Lexeme>),
    Malformed(Vec<Lexeme>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefineDirective {
    pub name: String,
    pub definition: MacroDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacroDefinition {
    ObjectLike {
        replacement: Vec<Lexeme>,
    },
    FunctionLike {
        parameters: Vec<String>,
        variadic: bool,
        replacement: Vec<Lexeme>,
    },
    Malformed {
        tokens: Vec<Lexeme>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VariadicKind {
    CStyle,
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionalDirective {
    pub expression: Vec<Lexeme>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroCandidate {
    pub name: String,
    pub parameters: Option<Vec<Vec<Lexeme>>>,
}

impl PreprocessorToken {
    pub fn as_source(&self) -> String {
        self.original.iter()
            .map(|lexeme| lexeme.as_source())
            .collect()
    }
}

pub fn parse_tokens(lexemes: Vec<Lexeme>) -> Vec<PreprocessorToken> {
    let mut tokens = Vec::new();
    let mut position: usize = 0;

    let mut non_whitespace_on_line = false;
    let mut last_char_was_backslash = false;
    let mut current_line = Vec::new();
    while let Some(current) = lexemes.get(position) {
        if *current == Lexeme::NewLine && !last_char_was_backslash {
            non_whitespace_on_line = false;
        }

        if *current == Lexeme::Backslash {
            last_char_was_backslash = true;
        }
        else {
            last_char_was_backslash = false;
        }

        if !non_whitespace_on_line && is_directive_line(&lexemes[position..]) {
            if !current_line.is_empty() {
                tokens.push(PreprocessorToken{
                    original: current_line,
                    kind: PreprocessorTokenKind::Text,
                });

                current_line = Vec::new();
            }

            let line = get_full_line(&lexemes, &mut position);
            tokens.push(parse_directive_line(line));
        }
        else if let Some(identifier) = eat_identifier(&lexemes, &mut position) {
            if !current_line.is_empty() {
                tokens.push(PreprocessorToken{
                    original: current_line,
                    kind: PreprocessorTokenKind::Text,
                });

                current_line = Vec::new();
            }

            non_whitespace_on_line = true;
            let token = parse_macro_candidate(&lexemes, &mut position, identifier);
            match token {
                Some(token) => tokens.push(token),
                None => {
                    current_line.push(current.clone());
                }
            }
        }
        else {
            non_whitespace_on_line &= !is_trivial(current);
            current_line.push(current.clone());
            position += 1;
        }
    }

    tokens
}

fn get_full_line(lexemes: &[Lexeme], position: &mut usize) -> Vec<Lexeme> {
    let mut seen_non_trivial = false;
    let mut line = Vec::new();

    while let Some(current) = lexemes.get(*position) {
        if matches!(current, Lexeme::NewLine) && !matches!(line.last(), Some(Lexeme::Backslash)) && seen_non_trivial {
            break;
        }

        if !is_trivial(current) {
            seen_non_trivial = true;
        }

        line.push(current.clone());
        *position += 1;
    }

    line
}

fn is_directive_line(line: &[Lexeme]) -> bool {
    first_non_trivial(line).is_some_and(|lexeme| matches!(lexeme, Lexeme::Hash))
}

fn first_non_trivial(line: &[Lexeme]) -> Option<&Lexeme> {
    line.iter().find(|lexeme| !is_trivial(lexeme))
}

fn is_trivial(lexeme: &Lexeme) -> bool {
    matches!(lexeme, Lexeme::Whitespace(_) | Lexeme::LineComment(_) | Lexeme::BlockComment(_) | Lexeme::NewLine)
}

fn skip_trivial(line: &[Lexeme], position: &mut usize) {
    while line.get(*position).map_or(false, is_trivial) {
        *position += 1;
    }
}

fn trim_trivial(lexemes: &[Lexeme]) -> &[Lexeme] {
    let start = lexemes
        .iter()
        .position(|lexeme| !is_trivial(lexeme))
        .unwrap_or(lexemes.len());

    let end = lexemes
        .iter()
        .rposition(|lexeme| !is_trivial(lexeme))
        .map(|position| position + 1)
        .unwrap_or(start);

    &lexemes[start..end]
}

fn eat_identifier(line: &[Lexeme], position: &mut usize) -> Option<String> {
    match line.get(*position)? {
        Lexeme::Identifier(value) => {
            *position += 1;
            Some(value.clone())
        }
        _ => None,
    }
}

fn parse_directive_line(line: Vec<Lexeme>) -> PreprocessorToken {
    let mut position: usize = 0;

    skip_trivial(&line, &mut position);

    if !matches!(line.get(position), Some(Lexeme::Hash)) {
        return PreprocessorToken{
            original: line,
            kind: PreprocessorTokenKind::Text,
        };
    }

    position += 1;
    skip_trivial(&line, &mut position);

    let Some(directive_name) = eat_identifier(&line, &mut position) else {
        return PreprocessorToken{
            original: line,
            kind: PreprocessorTokenKind::OtherDirective,
        };
    };

    match directive_name.as_str() {
        "include" => parse_include_directive(line, position),
        "define" => parse_define_directive(line, position),
        "undef" => parse_undef_directive(line, position),

        "if" => parse_if_directive(line, position),
        "ifdef" => parse_ifdef_directive(line, position),
        "ifndef" => parse_ifndef_directive(line, position),
        "elif" => parse_elif_directive(line, position),
        "elifdef" => parse_elifdef_directive(line, position),
        "elifndef" => parse_elifndef_directive(line, position),

        "else" => PreprocessorToken{
            original: line,
            kind: PreprocessorTokenKind::Else,
        },

        "endif" => PreprocessorToken{
            original: line,
            kind: PreprocessorTokenKind::EndIf,
        },

        "pragma" => parse_pragma_directive(line, position),

        _ => PreprocessorToken{
            original: line,
            kind: PreprocessorTokenKind::OtherDirective,
        },
    }
}

fn parse_include_directive(line: Vec<Lexeme>, position: usize) -> PreprocessorToken {
    let mut position = position;
    skip_trivial(&line, &mut position);

    let include_target = match line.get(position) {
        Some(Lexeme::StringLiteral(name)) => IncludeTarget::Quoted(name.trim_matches('"').to_string()),
        Some(Lexeme::Less) => parse_angle_include(&line, position),
        Some(Lexeme::Identifier(_)) => IncludeTarget::Macro(trim_trivial(&line[position..]).to_vec()),
        _ => IncludeTarget::Malformed(trim_trivial(&line[position..]).to_vec()),
    };

    PreprocessorToken {
        original: line,
        kind: PreprocessorTokenKind::Include(IncludeDirective {
            target: include_target
        }),
    }
}

fn parse_angle_include(line: &Vec<Lexeme>, position: usize) -> IncludeTarget {
    let mut position = position;
    position += 1;

    let mut angle_found = false;
    let mut str = String::new();
    while let Some(lexeme) = line.get(position) {
        position += 1;
        if matches!(lexeme, Lexeme::Greater) {
            angle_found = true;
            break;
        }

        str.push_str(lexeme.as_source());
    }

    if !angle_found {
        return IncludeTarget::Malformed(trim_trivial(&line[position..]).to_vec());
    }
    IncludeTarget::Angled(str)
}

fn parse_define_directive(line: Vec<Lexeme>, position: usize) -> PreprocessorToken {
    let mut position = position;
    skip_trivial(&line, &mut position);

    let Some(name) = eat_identifier(&line, &mut position) else {
        return PreprocessorToken {
            original: line,
            kind: PreprocessorTokenKind::OtherDirective,
        };
    };

    let definition = if matches!(line.get(position), Some(Lexeme::LParen)) {
        parse_function_like_macro_definition(&line, &mut position)
    } else {
        MacroDefinition::ObjectLike {
            replacement: get_macro_replacement(&line, position),
        }
    };

    PreprocessorToken {
        original: line,
        kind: PreprocessorTokenKind::Define(DefineDirective {
            name,
            definition
        }),
    }
}

fn get_macro_replacement(line: &[Lexeme], position: usize) -> Vec<Lexeme> {
    trim_trivial(&line[position..]).iter()
        .filter(|&lexeme| !matches!(lexeme, Lexeme::Backslash))
        .map(|lexeme| {
            match lexeme {
                Lexeme::NewLine => Lexeme::Whitespace(" ".to_string()),
                _ => lexeme.clone(),
            }
        })
        .collect()
}

fn parse_function_like_macro_definition(line: &[Lexeme], position: &mut usize) -> MacroDefinition {
    *position += 1;
    let mut parameters = Vec::new();

    let mut has_param_pack = false;
    loop {
        skip_trivial(line, position);

        match line.get(*position) {
            Some(Lexeme::RParen) => {
                *position += 1;
                break;
            }

            Some(Lexeme::Identifier(parameter)) => {
                parameters.push(parameter.clone());
                *position += 1;

                skip_trivial(line, position);

                match line.get(*position) {
                    Some(Lexeme::Comma) => {
                        *position += 1;
                    }
                    Some(Lexeme::RParen) => {
                        *position += 1;
                        break;
                    }
                    _ => {
                        return MacroDefinition::Malformed {
                            tokens: line.to_vec(),
                        };
                    }
                }
            }

            Some(Lexeme::Other(str)) if str == "..." => {
                *position += 1;
                has_param_pack = true;
            }

            _ => {
                return MacroDefinition::Malformed {
                    tokens: line.to_vec(),
                };
            }
        }
    }

    MacroDefinition::FunctionLike {
        parameters,
        variadic: has_param_pack,
        replacement: get_macro_replacement(line, *position),
    }
}

fn parse_undef_directive(line: Vec<Lexeme>, position: usize) -> PreprocessorToken {
    let mut position = position;
    skip_trivial(&line, &mut position);

    let Some(name) = eat_identifier(&line, &mut position) else {
        return PreprocessorToken {
            original: line,
            kind: PreprocessorTokenKind::OtherDirective,
        };
    };

    PreprocessorToken {
        original: line,
        kind: PreprocessorTokenKind::Undef(NameDirective {
            name
        }),
    }
}

fn parse_if_directive(line: Vec<Lexeme>, position: usize) -> PreprocessorToken {
    let trimmed = trim_trivial(&line[position..]).to_vec();
    PreprocessorToken {
        original: line,
        kind: PreprocessorTokenKind::If(ConditionalDirective {
            expression: trimmed,
        }),
    }
}

fn parse_ifdef_directive(line: Vec<Lexeme>, position: usize) -> PreprocessorToken {
    parse_name_directive(line, position, PreprocessorTokenKind::IfDef)
}

fn parse_ifndef_directive(line: Vec<Lexeme>, position: usize) -> PreprocessorToken {
    parse_name_directive(line, position, PreprocessorTokenKind::IfNDef)
}

fn parse_elif_directive(line: Vec<Lexeme>, position: usize) -> PreprocessorToken {
    let trimmed = trim_trivial(&line[position..]).to_vec();
    PreprocessorToken {
        original: line,
        kind: PreprocessorTokenKind::Elif(ConditionalDirective {
            expression: trimmed,
        }),
    }
}

fn parse_elifdef_directive(line: Vec<Lexeme>, position: usize) -> PreprocessorToken {
    parse_name_directive(line, position, PreprocessorTokenKind::Elifdef)
}

fn parse_elifndef_directive(line: Vec<Lexeme>, position: usize) -> PreprocessorToken {
    parse_name_directive(line, position, PreprocessorTokenKind::Elifndef)
}

fn parse_name_directive(line: Vec<Lexeme>, position: usize, build: impl FnOnce(NameDirective)-> PreprocessorTokenKind) -> PreprocessorToken {
    let mut position = position;
    skip_trivial(&line, &mut position);

    let Some(name) = eat_identifier(&line, &mut position) else {
        return PreprocessorToken {
            original: line,
            kind: PreprocessorTokenKind::OtherDirective,
        };
    };

    PreprocessorToken {
        original: line,
        kind: build(NameDirective {
            name,
        }),
    }
}

fn parse_pragma_directive(line: Vec<Lexeme>, position: usize) -> PreprocessorToken {
    let mut position = position;
    match eat_identifier(&line, &mut position).as_deref() {
        Some("once") => PreprocessorToken{
            original: line,
            kind: PreprocessorTokenKind::PragmaOnce
        },
        _ => PreprocessorToken{
            original: line,
            kind: PreprocessorTokenKind::OtherDirective,
        },
    }
}

fn parse_macro_candidate(lexemes: &[Lexeme], position: &mut usize, identifier: String) -> Option<PreprocessorToken> {
    let start = *position;
    if !matches!(lexemes.get(*position), Some(Lexeme::LParen)) {
        return Some(PreprocessorToken {
            original: vec!(Lexeme::Identifier(identifier.clone())),
            kind: PreprocessorTokenKind::MacroCandidate(MacroCandidate {
                name: identifier,
                parameters: None
            })
        });
    }

    let mut parameters = Vec::new();
    let mut current_parameter = Vec::new();
    let mut has_parameters = false;
    *position += 1;
    let mut paren_depth: usize = 1;
    while paren_depth > 0 && let Some(lexeme) = lexemes.get(*position) {
        *position += 1;
        match lexeme {
            Lexeme::LParen => paren_depth += 1,
            Lexeme::RParen => paren_depth -= 1,
            Lexeme::Comma if paren_depth == 1 => {
                parameters.push(current_parameter);
                current_parameter = Vec::new();
            }
            any if is_trivial(any) => {

            }
            _ => {
                current_parameter.push(lexeme.clone());
                has_parameters = true;
            }
        }
        *position += 1;
    }

    if paren_depth > 0 {
        return None;
    }

    if has_parameters {
        parameters.push(current_parameter);
    }

    Some(PreprocessorToken {
        original: iter::once(Lexeme::Identifier(identifier.clone()))
            .chain(lexemes[start..*position].iter().cloned())
            .collect(),
        kind: PreprocessorTokenKind::MacroCandidate(MacroCandidate {
            name: identifier,
            parameters: Some(parameters)
        })
    })
}