use std::io::{BufRead, Write};

pub struct IndentedWriter<W: Write> {
    writer: W,
    indent_level: u32,
    indent_size: u32,
    wrote_indent: bool
}

impl<W: Write> IndentedWriter<W> {
    pub fn new(writer: W) -> Self {
        Self::new_with_indent_size(writer, 4)
    }

    pub fn new_with_indent_size(writer: W, indent_size: u32) -> Self {
        Self {
            writer,
            indent_level: 0,
            indent_size,
            wrote_indent: false
        }
    }

    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    pub fn dedent(&mut self) {
        if self.indent_level == 0 {
            panic!("Cannot dedent when indent level is 0");
        }

        self.indent_level -= 1;
    }

    pub fn write_unindented(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if buf.contains(&b'\n') {
            self.wrote_indent = false;
        }
        self.writer.write(buf)
    }

    pub fn write_all_unindented(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if matches!(buf.last(), Some(&b'\n')) {
            self.wrote_indent = false;
        }
        self.writer.write_all(buf)
    }

    pub fn write_fmt_unindented(&mut self, fmt: std::fmt::Arguments) -> std::io::Result<()> {
         let formatted = fmt.to_string();
        self.write_all_unindented(formatted.as_bytes())
    }

    pub fn write_indent(&mut self) -> std::io::Result<()> {
        self.wrote_indent = true;
        for _ in 0..(self.indent_size * self.indent_level) {
            self.writer.write(&[b' '])?;
        }

        Ok(())
    }
}

impl<W: Write> Write for IndentedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut written = 0;
        if !self.wrote_indent {
            self.write_indent()?;
        }

        let mut index: usize = 0;
        for split_items in buf.split(|&b| b == b'\n') {
            if index > 0 {
                written += self.writer.write(b"\n")?;
                if split_items.len() > 0 {
                    self.write_indent()?;
                }
                self.wrote_indent = false;
            }

            written += self.writer.write(split_items)?;
            index += 1;
        }

        Ok(written)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}