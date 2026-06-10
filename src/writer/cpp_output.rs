use crate::config::{Config, ConfigIncludePath};
use crate::writer::IndentedWriter;
use std::io::{Read, Write};
use tempfile::NamedTempFile;
use clang::{Clang, Index};
use crate::parser::extractor::extract_symbols;
use crate::writer::symbols::SymbolWriteContext;

impl Config {
    pub fn output_module(&self) -> anyhow::Result<()> {
        let mut includes = NamedTempFile::new()?;
        for header in &self.headers.library_headers {
            match header {
                ConfigIncludePath::Unconditional(file) => {
                    let header_file = file.display();
                    includes.write_fmt(format_args!("#include <{header_file}>\n"))?;
                }
                ConfigIncludePath::Conditional { path, if_defined } => {
                    let header_file = path.display();
                    includes.write_fmt(format_args!("#ifdef {if_defined}\n#include <{header_file}>\n#endif\n"))?;
                }
            }
        }

        let clang = Clang::new()
            .map_err(|e| anyhow::anyhow!("Clang initialization error: {}", e))?;
        let index = Index::new(&clang, false, false);

        let mut arguments = vec!["-x".to_string(), "c++".to_string(), "-std=c++20".to_string()];
        for path in &self.headers.include_dirs {
            arguments.push(format!("-I{}", path.display()));
        }

        let translation_unit = index.parser(includes.path())
            .arguments(&arguments)
            .skip_function_bodies(true)
            .detailed_preprocessing_record(true)
            .retain_excluded_conditional_blocks(true)
            .parse()?;

        for diagnostic in translation_unit.get_diagnostics() {
            println!("{}", diagnostic);
        }

        //let translation_unit = TranslationUnit::new(self, &includes)?;

        let file = std::fs::File::create(&self.module.output_path)?;
        let mut writer = IndentedWriter::new(file);

        writer.write_all(b"module;\n\n")?;
        let mut include_bytes = Vec::new();
        includes.read_to_end(&mut include_bytes)?;
        writer.write_all(&include_bytes)?;
        let name = &self.module.name;
        writer.write_fmt(format_args!("\nexport module {name};\n"))?;

        /*
        if translation_unit.has_macros() {
            writer.write_all(b"\n\n")?;
            writer.write_all(b"/*\nDiscovered Macros:\n")?;
            for macro_name in translation_unit.macros() {
                writer.write_fmt(format_args!("- {macro_name}\n"))?;
            }
            writer.write_all(b"*/\n")?;
        }
         */

        let mut symbol_context = SymbolWriteContext::new(&mut writer);
        let symbols = extract_symbols(&translation_unit);
        symbol_context.emit_symbols(&symbols)?;

        Ok(())
    }
}