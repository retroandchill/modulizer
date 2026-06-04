use std::io::Write;
use crate::config::Config;
use crate::writer::IndentedWriter;


impl Config {
    pub fn output_module(&self) -> anyhow::Result<()> {
        let file = std::fs::File::create(&self.module.output_path)?;
        let mut writer = IndentedWriter::new(file);

        writer.write_all(b"module;\n\n")?;

        let name = &self.module.name;
        writer.write_fmt(format_args!("export module {name};\n"))?;

        Ok(())
    }
}