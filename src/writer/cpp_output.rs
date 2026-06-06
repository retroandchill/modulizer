use std::fmt::Write as FmtWrite;
use std::io::Write;
use crate::config::Config;
use crate::preprocessor::preprocess;
use crate::writer::IndentedWriter;


impl Config {
    pub fn output_module(&self) -> anyhow::Result<()> {
        let file = std::fs::File::create(&self.module.output_path)?;
        let mut writer = IndentedWriter::new(file);

        let mut includes = String::new();

        for header in &self.headers.library_headers {
            let header_file = header.display();
            includes.write_fmt(format_args!("#include <{header_file}>;\n"))?;
        }

        let preprocessed = preprocess(&includes, self)?;
        let mut expansion = std::fs::File::create("expanded.cpp")?;
        expansion.write(preprocessed.source.as_bytes())?;

        writer.write_all(b"module;\n\n")?;
        writer.write_all(includes.as_bytes())?;
        let name = &self.module.name;
        writer.write_fmt(format_args!("\nexport module {name};\n"))?;

        Ok(())
    }
}