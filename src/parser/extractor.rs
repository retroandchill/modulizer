use clang::{Entity, EntityKind, TranslationUnit};
use crate::parser::symbols::{Namespace, Symbol, SymbolKind};
use itertools::Itertools;

pub fn extract_symbols(translation_unit: &TranslationUnit) -> Vec<Symbol> {
    let mut symbols: Vec<Symbol> = Vec::new();
    extract_entity_symbols(translation_unit.get_entity(), &mut symbols);
    symbols
}

fn extract_entity_symbols(entity: Entity, symbols: &mut Vec<Symbol>) {
    let kind = entity.get_kind();
    println!("{:?}", kind);
    match kind {
        EntityKind::TranslationUnit => {
            for child in entity.get_children() {
                extract_entity_symbols(child, symbols);
            }
        }
        EntityKind::Namespace => {
            let Some(name) = entity.get_name() else {
                return;
            };
            symbols.push(Symbol {
                name,
                guards: Vec::new(),
                kind: SymbolKind::Namespace(Namespace {
                    is_inline: entity.is_inline_namespace(),
                    symbols: extract_namespace_symbols(&entity),
                }),
            })
        }
        EntityKind::ClassDecl
        | EntityKind::StructDecl
        | EntityKind::FunctionDecl
        | EntityKind::VarDecl
        | EntityKind::UnionDecl
        | EntityKind::TypedefDecl
        | EntityKind::TypeAliasDecl
        | EntityKind::TypeAliasTemplateDecl
        | EntityKind::ClassTemplate
        | EntityKind::FunctionTemplate
        | EntityKind::ConceptDecl => symbols.push(Symbol {
            name: entity.get_name().unwrap(),
            guards: Vec::new(),
            kind: SymbolKind::ExportableSymbol,
        }),
        EntityKind::NamespaceAlias => {
            let target = entity.get_children().iter().filter_map(|c| c.get_name())
                .join("::");
            symbols.push(Symbol {
                name: entity.get_name().unwrap(),
                guards: Vec::new(),
                kind: SymbolKind::NamespaceAlias(target)
            })
        }
        EntityKind::UsingDirective => {
            let target = entity.get_children().iter().filter_map(|c| c.get_name())
                .join("::");
            symbols.push(Symbol {
                name: target,
                guards: Vec::new(),
                kind: SymbolKind::UsingNamespace
            })
        }
        EntityKind::UsingDeclaration => {
            let target = entity.get_children().iter().filter_map(|c| c.get_name())
                .join("::");
            symbols.push(Symbol {
                name: target,
                guards: Vec::new(),
                kind: SymbolKind::UsingDeclaration
            })
        }
        _ => {}
    }
}

fn extract_namespace_symbols(entity: &Entity) -> Vec<Symbol> {
    let mut symbols: Vec<Symbol> = Vec::new();

    for child in entity.get_children() {
        extract_entity_symbols(child, &mut symbols);
    }

    symbols
}
