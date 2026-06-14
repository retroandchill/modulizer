use logos::Logos;
use modulizer::parser::grammar::tokens::{GuardedToken, Token};
use modulizer::parser::symbols::{Symbol, SymbolKind, parse_symbols};
use std::assert_matches;
use std::rc::Rc;

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
    assert_declarations(
        &symbols,
        &[
            "operator+",
            "operator new",
            "operator new[]",
            "operator delete",
            "operator delete[]",
            "operator co_await",
            "operator[]",
            "operator()",
        ],
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

#[test]
fn can_parse_inline_type_declarations() {
    let code = "typedef struct Foo_ { int x; } Foo;
                          class Bar* get_bar();
                          enum class Bax : std::int32_t values[3];";
    let tokens = lex(code);
    let guarded = to_guarded_tokens(&tokens);
    let result = parse_symbols(&guarded);
    assert!(result.is_ok());
    let symbols = result.unwrap();
    assert_declarations(&symbols, &["Foo", "get_bar", "values"]);
}

#[test]
fn can_parse_extern_functions() {
    let code = "extern \"C\" int CFunction();
                         extern \"C\"
                         {
                             void CFunction2();
                         }";
    let tokens = lex(code);
    let guarded = to_guarded_tokens(&tokens);
    let result = parse_symbols(&guarded);
    assert!(result.is_ok());
    let symbols = result.unwrap();
    assert_eq!(symbols.len(), 2);
    let symbol1 = &symbols[0];
    assert_eq!(symbol1.name, "CFunction");
    assert_matches!(&symbol1.kind, SymbolKind::ExportableSymbol);
    let symbol2 = &symbols[1];
    assert_eq!(symbol2.name, "");
    let SymbolKind::ExternBlock(externed_symbols) = &symbol2.kind else {
        panic!("Expected ExternBlock, got {:?}", symbol2.kind);
    };
    assert_eq!(externed_symbols.len(), 1);
    assert_eq!(externed_symbols[0].name, "CFunction2");
    assert_matches!(&externed_symbols[0].kind, SymbolKind::ExportableSymbol);
}
