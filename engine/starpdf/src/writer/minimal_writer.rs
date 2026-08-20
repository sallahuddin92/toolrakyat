use std::io::{self, Write};

pub struct MinimalWriter;

impl MinimalWriter {
    /// Generates a minimal, syntactically standard single-page PDF document.
    pub fn create_minimal_pdf(content_text: &str) -> io::Result<Vec<u8>> {
        let mut buf = Vec::with_capacity(1024);

        // 1. Header
        buf.write_all(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n")?;

        let mut offsets = Vec::new();
        offsets.push(0u64); // 0 entry in xref is free

        // Object 1: Catalog
        offsets.push(buf.len() as u64);
        buf.write_all(b"1 0 obj\n<<\n  /Type /Catalog\n  /Pages 2 0 R\n>>\nendobj\n")?;

        // Object 2: Pages
        offsets.push(buf.len() as u64);
        buf.write_all(b"2 0 obj\n<<\n  /Type /Pages\n  /Kids [3 0 R]\n  /Count 1\n>>\nendobj\n")?;

        // Object 3: Page
        offsets.push(buf.len() as u64);
        buf.write_all(
            b"3 0 obj\n<<\n  /Type /Page\n  /Parent 2 0 R\n  /MediaBox [0 0 612 792]\n  /Contents 4 0 R\n>>\nendobj\n",
        )?;

        // Object 4: Contents Stream
        let stream_bytes = format!(
            "BT\n  /F1 24 Tf\n  100 700 Td\n  ({}) Tj\nET\n",
            content_text.replace('(', "\\(").replace(')', "\\)")
        )
        .into_bytes();

        offsets.push(buf.len() as u64);
        writeln!(
            buf,
            "4 0 obj\n<<\n  /Length {}\n>>\nstream",
            stream_bytes.len()
        )?;
        buf.write_all(&stream_bytes)?;
        buf.write_all(b"\nendstream\nendobj\n")?;

        // 2. XRef Table
        let xref_offset = buf.len() as u64;
        writeln!(buf, "xref\n0 {}", offsets.len())?;
        writeln!(buf, "0000000000 65535 f ")?;
        for &offset in &offsets[1..] {
            writeln!(buf, "{:010} 00000 n ", offset)?;
        }

        // 3. Trailer & StartXRef
        writeln!(
            buf,
            "trailer\n<<\n  /Size {}\n  /Root 1 0 R\n>>\nstartxref\n{}\n%%EOF",
            offsets.len(),
            xref_offset
        )?;

        Ok(buf)
    }
}
