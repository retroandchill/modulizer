use std::fmt::Write as FmtWrite;
use std::io::Write;
use crate::config::Config;
use crate::parser::translation::TranslationUnit;
use crate::writer::IndentedWriter;


impl Config {
    pub fn output_module(&self) -> anyhow::Result<()> {
        let file = std::fs::File::create(&self.module.output_path)?;
        let mut writer = IndentedWriter::new(file);

        let mut includes = String::new();

        for header in &self.headers.library_headers {
            let header_file = header.display();
            includes.write_fmt(format_args!("#include <{header_file}>\n"))?;
        }

        let translation_unit = TranslationUnit::new(self, &includes)?;
        let mut expansion = std::fs::File::create("expanded.cpp")?;
        expansion.write_fmt(format_args!("{}", translation_unit))?;

        writer.write_all(b"module;\n\n")?;
        writer.write_all(includes.as_bytes())?;
        let name = &self.module.name;
        writer.write_fmt(format_args!("\nexport module {name};\n"))?;

        Ok(())
    }
}