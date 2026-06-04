use std::path::PathBuf;

pub enum IncludeStyle {
    Quoted,
    Angled
}

pub struct IncludeDirective {
    pub style: IncludeStyle,
    pub target: PathBuf,
    pub line: usize,
}

impl IncludeDirective {
    fn parse(line: &str, line_number: usize) -> anyhow::Result<IncludeDirective> {
        Err(anyhow::anyhow!("Include directive parsing not implemented"))
    }
}