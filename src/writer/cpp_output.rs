use crate::config::{IncludePath, Options};
use crate::parser::translation::TranslationUnit;
use crate::writer::IndentedWriter;
use crate::writer::symbols::SymbolWriteContext;
use std::fmt::Write as FmtWrite;
use std::io::Write;

impl Options {
    pub fn output_module(&self) -> anyhow::Result<()> {
        let file = std::fs::File::create(&self.output_path)?;
        let mut writer = IndentedWriter::new(file);

        let mut includes = String::new();

        for header in &self.library_headers {
            match header {
                IncludePath::Unconditional(file) => {
                    let header_file = file.display();
                    includes.write_fmt(format_args!("#include <{header_file}>\n"))?;
                }
                IncludePath::IfDefined { path, if_defined } => {
                    let header_file = path.display();
                    includes.write_fmt(format_args!(
                        "#ifdef {if_defined}\n#include <{header_file}>\n#endif\n"
                    ))?;
                }
                IncludePath::Conditioned { path, condition } => {
                    let header_file = path.display();
                    includes.write_fmt(format_args!(
                        "#if {condition}\n#include <{header_file}>\n#endif\n"
                    ))?;
                }
            }
        }

        let translation_unit = TranslationUnit::new(self, &includes)?;

        writer.write_all(b"module;\n\n")?;
        writer.write_all(includes.as_bytes())?;
        let name = &self.name;
        writer.write_fmt(format_args!("\nexport module {name};\n"))?;

        if translation_unit.has_macros() {
            writer.write_all(b"\n\n")?;
            writer.write_all(b"/*\nDiscovered Macros:\n")?;
            for macro_name in translation_unit.macros() {
                writer.write_fmt(format_args!("- {macro_name}\n"))?;
            }
            writer.write_all(b"*/\n")?;
        }

        let mut symbol_context = SymbolWriteContext::new(&mut writer);
        symbol_context.emit_symbols(translation_unit.symbols())?;

        Ok(())
    }
}
