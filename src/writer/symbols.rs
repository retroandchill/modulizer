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
            self.emit_symbol(symbol, "")?;
        }

        self.update_guards(&[])
    }

    fn emit_symbol(
        &mut self,
        symbol: &Symbol,
        namespace_name: &str,
    ) -> std::io::Result<()> {
        self.update_guards(&symbol.guards)?;

        match &symbol.kind {
            SymbolKind::Namespace(namespace) => {
                if namespace.is_inline {
                    self.writer.write_all(b"inline ")?;
                }

                self.writer
                    .write_fmt(format_args!("namespace {}\n", namespace.name))?;
                self.writer.write_all(b"{\n")?;
                self.writer.indent();

                for symbol in &namespace.symbols {
                    self.emit_symbol(symbol, &namespace.name)?;
                }

                self.update_guards(&symbol.guards)?;

                self.writer.dedent();
                self.writer.write_all(b"}\n")?;
            }
            SymbolKind::Class { name }
            | SymbolKind::Struct { name }
            | SymbolKind::Union { name }
            | SymbolKind::Typedef { name }
            | SymbolKind::Function { name }
            | SymbolKind::Variable { name }
            | SymbolKind::Concept { name }
            | SymbolKind::Enum { name, .. } => {
                self.writer.write_fmt(format_args!("using {namespace_name}::{name};\n"))?;
            }
            SymbolKind::Using { name, namespace } => {
                if *namespace {
                    self.writer.write_fmt(format_args!("using namespace {name};\n"))?;
                }
                else {
                    self.writer.write_fmt(format_args!("using {namespace_name}::{name};\n"))?;
                }
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
                        for token in expression {
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
                        for token in expression {
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
