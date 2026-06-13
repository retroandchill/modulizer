use crate::parser::grammar::PreprocessorGuard;
use crate::parser::preprocessor::ConditionalDirective;
use crate::parser::symbols::{Symbol, SymbolKind};
use crate::writer::IndentedWriter;
use std::collections::VecDeque;
use std::io::Write;

pub struct SymbolWriteContext<'a, W: Write> {
    writer: &'a mut IndentedWriter<W>,
    guards: VecDeque<PreprocessorGuard>,
}

impl<'a, W: Write> SymbolWriteContext<'a, W> {
    pub fn new(writer: &'a mut IndentedWriter<W>) -> Self {
        Self {
            writer,
            guards: VecDeque::new(),
        }
    }

    pub fn emit_symbols<'b>(&mut self, symbols: impl IntoIterator<Item = &'b Symbol>) -> std::io::Result<()> {
        for symbol in symbols {
            self.emit_symbol(symbol, "", "")?;
        }

        self.update_guards(&[])
    }

    fn emit_symbol(
        &mut self,
        symbol: &Symbol,
        namespace_name: &str,
        full_scope: &str
    ) -> std::io::Result<()> {
        self.update_guards(&symbol.guards)?;

        let name = symbol.name.as_str();
        let current_scope = if full_scope.is_empty() {
            full_scope.to_string()
        } else {
            format!("{}::{}", full_scope, name)
        };
        match &symbol.kind {
            SymbolKind::ExternBlock(symbols) => {
                for symbol in symbols {
                    self.emit_symbol(symbol, namespace_name, &current_scope)?;
                }
            }
            SymbolKind::Namespace(namespace) => {
                if !namespace.is_inline {
                    self.writer
                        .write_fmt(format_args!("namespace {}\n", name))?;
                    self.writer.write_all(b"{\n")?;
                    self.writer.indent();
                }

                for symbol in &namespace.symbols {
                    self.emit_symbol(symbol, name, &current_scope)?;
                }

                self.update_guards(&symbol.guards)?;

                if !namespace.is_inline {
                    self.writer.dedent();
                    self.writer.write_all(b"}\n")?;
                }
            }
            SymbolKind::ExportableSymbol => {
                self.writer.write_fmt(format_args!("export using {namespace_name}::{name};\n"))?;
            }
            SymbolKind::UsingDeclaration => {
                self.writer.write_fmt(format_args!("export using {name};\n"))?;
            }
            SymbolKind::UsingNamespace => {
                self.writer.write_fmt(format_args!("export using namespace {name};\n"))?;
            }
            SymbolKind::NamespaceAlias(target) => {
                self.writer.write_fmt(format_args!("export namespace {name} = {target};\n"))?;
            }
        }

        Ok(())
    }

    fn update_guards(&mut self, new_guards: &[PreprocessorGuard]) -> std::io::Result<()> {
        let shared_prefix_len = self
            .guards
            .iter()
            .zip(new_guards.iter())
            .take_while(|(old, new)| old == new)
            .count();

        let can_switch_to_else = self.guards.len() == shared_prefix_len + 1
            && new_guards.len() == shared_prefix_len + 1
            && matches!(self.guards.back(), Some(PreprocessorGuard::Conditional(_)))
            && matches!(
                new_guards.get(shared_prefix_len),
                Some(
                    PreprocessorGuard::Conditional(
                        ConditionalDirective::Elif { .. }
                            | ConditionalDirective::Elifdef { .. }
                            | ConditionalDirective::Elifndef { .. }
                    ) | PreprocessorGuard::Else
                )
            );

        if can_switch_to_else {
            // emit "#else"
            self.guards.pop_back();
            self.guards.push_back(new_guards.last().unwrap().clone());
            return Ok(());
        }

        while self.guards.len() > shared_prefix_len {
            self.guards.pop_back();
            self.writer.write_all_unindented(b"#endif\n")?;
        }

        for guard in &new_guards[shared_prefix_len..] {
            match guard {
                PreprocessorGuard::Conditional(condition) => match condition {
                    ConditionalDirective::If { expression } => {
                        self.writer.write_all_unindented(b"#if ")?;
                        for token in expression.iter() {
                            self.writer
                                .write_fmt_unindented(format_args!("{}", token))?;
                        }
                        self.writer.write_all_unindented(b"\n")?;
                    }
                    ConditionalDirective::Ifdef { name } => {
                        self.writer
                            .write_fmt_unindented(format_args!("#ifdef {}\n", name))?;
                    }
                    ConditionalDirective::Ifndef { name } => {
                        self.writer
                            .write_fmt_unindented(format_args!("#ifndef {}\n", name))?;
                    }
                    ConditionalDirective::Elif { expression } => {
                        self.writer.write_all_unindented(b"#if ")?;
                        for token in expression.iter() {
                            self.writer
                                .write_fmt_unindented(format_args!("{}", token))?;
                        }
                        self.writer.write_all_unindented(b"\n")?;
                    }
                    ConditionalDirective::Elifdef { name } => {
                        self.writer
                            .write_fmt_unindented(format_args!("#elifdef {}\n", name))?;
                    }
                    ConditionalDirective::Elifndef { name } => {
                        self.writer
                            .write_fmt_unindented(format_args!("#elifndef {}\n", name))?;
                    }
                },
                PreprocessorGuard::Else => {
                    self.writer.write_all_unindented(b"#else\n")?;
                }
            }

            self.guards.push_back(guard.clone());
        }

        Ok(())
    }
}
