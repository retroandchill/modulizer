use std::fmt;
use crate::parser::grammar::{GuardedToken, PreprocessorGuard, Token};
use std::fmt::Write;
use std::thread::Scope;

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

pub enum DeclarationTerminator {
    Semicolon,
    ClosingBrace,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
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
        let mut symbols = Vec::new();

        while !self.is_at_end() {
            if let Some(symbol) = self.parse_symbol() {
                symbols.push(symbol);
            }
            else {
                self.advance();
            }
        }

        symbols
    }

    fn parse_symbol(&mut self) -> Option<Symbol> {
        match self.peek()?.token {
            Token::Inline => {
                if self.check_next(&Token::Namespace) {
                    self.advance();
                    return self.parse_namespace(true);
                }

                None
            }
            Token::Namespace => self.parse_namespace(false),
            Token::Class => self.parse_class(),
            Token::Struct => self.parse_struct(),
            Token::Union => self.parse_union(),
            Token::Enum => self.parse_enum(),
            Token::Using => self.parse_using(),
            Token::Typedef => self.parse_typedef(),
            Token::Concept => self.parse_concept(),
            _ => None,
        }
    }

    fn parse_namespace(&mut self, is_inline: bool) -> Option<Symbol> {
        let start = self.expect(Token::Namespace)?;

        if let Some(Token::Identifier(name)) = self.peek().map(|guarded| guarded.token) && self.check_next(&Token::Equal) {
            self.advance();
            self.expect(Token::Equal)?;
            let target = self.parse_qualified_name()?.join("::");
            return Some(Symbol {
                name: name.clone(),
                guards: start.guards.to_vec(),
                kind: SymbolKind::NamespaceAlias(target),
            });
        }

        let Some(mut names) = self.parse_qualified_name() else {
            self.skip_attributes();
            self.expect(Token::LBrace)?;
            self.skip_scope();
            self.expect(Token::RBrace)?;

            return None;
        };
        names.reverse();

        self.skip_attributes();
        self.expect(Token::LBrace)?;
        let children = self.parse_scope();
        self.expect(Token::RBrace)?;

        Some(extract_namespace(&start.guards, is_inline, names, children))
    }

    fn parse_class(&mut self) -> Option<Symbol> {
        let declaration = self.expect(Token::Class)?;
        self.parse_class_or_struct(declaration)
    }

    fn parse_struct(&mut self) -> Option<Symbol> {
        let declaration = self.expect(Token::Struct)?;
        self.parse_class_or_struct(declaration)
    }

    fn parse_class_or_struct(&mut self, declaration: GuardedToken<'tok>) -> Option<Symbol> {
        let name = match self.peek()?.token {
            Token::Identifier(name) => {
                self.advance();
                Some(name)
            },
            _ => None,
        };

        // Skip over the base class list if present
        self.skip_optional_base_class_list();
        self.skip_optional_scope();
        self.expect(Token::Semicolon)?;

        name.map(|name| Symbol {
            name: name.clone(),
            guards: declaration.guards.to_vec(),
            kind: SymbolKind::ExportableSymbol,
        })
    }

    fn skip_optional_base_class_list(&mut self) {
        while !self.check(&Token::Semicolon) && !self.check(&Token::LBrace) {
            self.advance();
        }
    }

    fn parse_union(&mut self) -> Option<Symbol> {
        let declaration = self.expect(Token::Union)?;
        let name = match self.peek()?.token {
            Token::Identifier(name) => {
                self.advance();
                Some(name)
            },
            _ => None,
        };

        self.skip_optional_scope();
        self.expect(Token::Semicolon)?;

        name.map(|name| Symbol {
            name: name.clone(),
            guards: declaration.guards.to_vec(),
            kind: SymbolKind::ExportableSymbol,
        })
    }

    fn parse_enum(&mut self) -> Option<Symbol> {
        let declaration = self.expect(Token::Enum)?;

        if self.check(&Token::Class) || self.check(&Token::Struct) {
            self.advance();
        }
        self.parse_class_or_struct(declaration)
    }
    fn parse_using(&mut self) -> Option<Symbol> {
        self.expect(Token::Using)?;

        let next = self.peek()?;
        match next.token {
            Token::Namespace => {
                self.advance();
                let names = self.parse_qualified_name()?;
                self.expect(Token::Semicolon)?;
                Some(Symbol {
                    name: names.join("::"),
                    guards: next.guards.to_vec(),
                    kind: SymbolKind::UsingNamespace,
                })
            }
            Token::Identifier(name) => {
                if self.check_next(&Token::Equal) {
                    self.skip_until_semicolon();
                    Some(Symbol {
                        name: name.clone(),
                        guards: next.guards.to_vec(),
                        kind: SymbolKind::ExportableSymbol,
                    })
                } else {
                    let names = self.parse_qualified_name()?;
                    self.expect(Token::Semicolon)?;
                    Some(Symbol {
                        name: names.join("::"),
                        guards: next.guards.to_vec(),
                        kind: SymbolKind::UsingDeclaration,
                    })
                }
            }
            _ => {
                None
            }
        }
    }

    fn parse_typedef(&mut self) -> Option<Symbol> {
        None
    }

    fn parse_concept(&mut self) -> Option<Symbol> {
        None
    }

    fn parse_scope(&mut self) -> Vec<Symbol> {
        let mut children = Vec::new();

        while !self.is_at_end() && !self.check(&Token::RBrace) {
            if let Some(symbol) = self.parse_symbol() {
                children.push(symbol);
            } else {
                self.advance();
            }
        }

        children
    }

    fn skip_optional_scope(&mut self) {
        if self.check(&Token::LBrace) {
            self.advance();
            self.skip_scope();
            self.advance();
        }
    }

    fn skip_scope(&mut self) {
        let mut depth = 1usize;
        while let Some(token) = self.peek() {
            match token.token {
                Token::LBrace => {
                    depth += 1;
                }
                Token::RBrace => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            self.advance();
        }
    }

    fn skip_until_semicolon(&mut self) {
        let mut depth = 1usize;
        while !self.is_at_end() && !self.check(&Token::Semicolon) && depth > 0 {
            match &self.tokens[self.index].token {
                Token::LBrace => depth += 1,
                Token::RBrace => depth = depth.saturating_sub(1),
                _ => {}
            }
            self.advance();
        }
    }

    fn parse_qualified_name(&mut self) -> Option<Vec<String>> {
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

        Some(parts)
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

    fn expect(&mut self, expected: Token) -> Option<GuardedToken<'tok>> {
        if !(self.check(&expected)) {
            return None;
        }

        let current = self.tokens[self.index].clone();
        self.advance();
        Some(current)
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

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.index).map(|guarded| guarded.token)
    }

    fn peek(&self) -> Option<GuardedToken<'tok>> {
        self.tokens.get(self.index).map(|guarded| guarded.clone())
    }

    fn peek_next(&self) -> Option<GuardedToken<'tok>> {
        self.tokens.get(self.index + 1).map(|guarded| guarded.clone())
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