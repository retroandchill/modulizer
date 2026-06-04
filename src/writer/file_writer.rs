use std::io::Write;

pub struct IndentedWriter<W: Write> {
    writer: W,
    indent_level: u32,
    indent_size: u32
}

impl<W: Write> IndentedWriter<W> {
    pub fn new(writer: W) -> Self {
        Self::new_with_indent_size(writer, 4)
    }

    pub fn new_with_indent_size(writer: W, indent_size: u32) -> Self {
        Self {
            writer,
            indent_level: 0,
            indent_size
        }
    }

    pub fn indent(&mut self) {
        self.indent_level += 1;
    }

    pub fn dedent(&mut self) {
        self.indent_level -= 1;
    }

    pub fn enter_indent_scope(&mut self) -> IndentScope<'_, W> {
        self.indent();
        IndentScope { writer: self }
    }
}

impl<W: Write> Write for IndentedWriter<W> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        for _ in 0..(self.indent_size * self.indent_level) {
            self.writer.write(&[b' '])?;
        }
        self.writer.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.writer.flush()
    }
}

pub struct IndentScope<'a, W: Write> {
    writer: &'a mut IndentedWriter<W>
}

impl<'a, W: Write> Drop for IndentScope<'a, W> {
    fn drop(&mut self) {
        self.writer.dedent()
    }
}