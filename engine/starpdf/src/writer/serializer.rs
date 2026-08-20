use crate::syntax::object::PdfObject;
use std::io::{self, Write};

pub struct Serializer;

impl Serializer {
    pub fn write_object<W: Write>(w: &mut W, obj: &PdfObject) -> io::Result<()> {
        match obj {
            PdfObject::Null => write!(w, "null"),
            PdfObject::Bool(true) => write!(w, "true"),
            PdfObject::Bool(false) => write!(w, "false"),
            PdfObject::Integer(i) => write!(w, "{i}"),
            PdfObject::Real(r) => {
                if r.fract() == 0.0 {
                    write!(w, "{r:.1}")
                } else {
                    write!(w, "{r}")
                }
            }
            PdfObject::Name(n) => Self::write_name(w, n),
            PdfObject::String(bytes) => Self::write_literal_string(w, bytes),
            PdfObject::Array(arr) => {
                write!(w, "[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(w, " ")?;
                    }
                    Self::write_object(w, item)?;
                }
                write!(w, "]")
            }
            PdfObject::Dictionary(dict) => {
                write!(w, "<<\n")?;
                for (key, val) in dict {
                    write!(w, "  ")?;
                    Self::write_name(w, key)?;
                    write!(w, " ")?;
                    Self::write_object(w, val)?;
                    write!(w, "\n")?;
                }
                write!(w, ">>")
            }
            PdfObject::Stream(s) => {
                let mut dict = s.dict.clone();
                dict.insert(
                    "Length".to_string(),
                    PdfObject::Integer(s.data.len() as i64),
                );
                Self::write_object(w, &PdfObject::Dictionary(dict))?;
                write!(w, "\nstream\n")?;
                w.write_all(&s.data)?;
                write!(w, "\nendstream")
            }
            PdfObject::Reference(r) => write!(w, "{} {} R", r.number, r.generation),
        }
    }

    pub fn write_name<W: Write>(w: &mut W, name: &str) -> io::Result<()> {
        write!(w, "/")?;
        for &b in name.as_bytes() {
            if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || b == b'.' {
                w.write_all(&[b])?;
            } else {
                write!(w, "#{:02X}", b)?;
            }
        }
        Ok(())
    }

    pub fn write_literal_string<W: Write>(w: &mut W, bytes: &[u8]) -> io::Result<()> {
        write!(w, "(")?;
        for &b in bytes {
            match b {
                b'(' => write!(w, "\\(")?,
                b')' => write!(w, "\\)")?,
                b'\\' => write!(w, "\\\\")?,
                b'\n' => write!(w, "\\n")?,
                b'\r' => write!(w, "\\r")?,
                b'\t' => write!(w, "\\t")?,
                _ => w.write_all(&[b])?,
            }
        }
        write!(w, ")")
    }

    pub fn to_bytes(obj: &PdfObject) -> Vec<u8> {
        let mut buf = Vec::new();
        let _ = Self::write_object(&mut buf, obj);
        buf
    }
}
