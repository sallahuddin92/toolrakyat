use std::collections::BTreeMap;
use std::error::Error;
use std::io::Write as _;
use std::path::PathBuf;

fn classic_pdf(objects: &[(u64, &str)], trailer_extra: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut pdf = b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n".to_vec();
    let mut offsets = BTreeMap::new();
    for (number, body) in objects {
        offsets.insert(*number, pdf.len());
        write!(&mut pdf, "{number} 0 obj\n{body}\nendobj\n")?;
    }
    let xref_offset = pdf.len();
    let size = objects.iter().map(|(number, _)| *number).max().unwrap_or(0) + 1;
    write!(&mut pdf, "xref\n0 {size}\n0000000000 65535 f \n")?;
    for number in 1..size {
        if let Some(offset) = offsets.get(&number) {
            writeln!(&mut pdf, "{offset:010} 00000 n ")?;
        } else {
            writeln!(&mut pdf, "0000000000 00000 f ")?;
        }
    }
    write!(
        &mut pdf,
        "trailer\n<< /Size {size} /Root 1 0 R /ID [<0011223344556677> <8899AABBCCDDEEFF>] {trailer_extra} >>\nstartxref\n{xref_offset}\n%%EOF\n"
    )?;
    Ok(pdf)
}

fn base_objects<'a>() -> Vec<(u64, &'a str)> {
    vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents 4 0 R >>"),
        (4, "<< /Length 36 >>\nstream\nBT /F1 12 Tf 72 720 Td (v0.11) Tj ET\nendstream"),
    ]
}

fn signed(valid_range: bool) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut objects = base_objects();
    objects[0].1 = "<< /Type /Catalog /Pages 2 0 R /AcroForm 7 0 R >>";
    objects[2].1 = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents 4 0 R /Annots [5 0 R] >>";
    objects.extend([
        (5, "<< /Type /Annot /Subtype /Widget /FT /Sig /T (SyntheticSignature) /Rect [72 620 300 670] /P 3 0 R /V 6 0 R >>"),
        (
            6,
            if valid_range {
                "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /ByteRange [0 64 128 64] /Contents <00112233445566778899AABBCCDDEEFF> /M (D:20260820190000+08'00') >>"
            } else {
                "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /ByteRange [0 200 100 80] /Contents <00112233> >>"
            },
        ),
        (7, "<< /Fields [5 0 R] /SigFlags 3 >>"),
    ]);
    classic_pdf(&objects, "")
}

fn signature_field_only() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut objects = base_objects();
    objects[0].1 = "<< /Type /Catalog /Pages 2 0 R /AcroForm 6 0 R >>";
    objects[2].1 = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents 4 0 R /Annots [5 0 R] >>";
    objects.extend([
        (5, "<< /Type /Annot /Subtype /Widget /FT /Sig /T (UnsignedSignatureField) /Rect [72 620 300 670] /P 3 0 R >>"),
        (6, "<< /Fields [5 0 R] /SigFlags 1 >>"),
    ]);
    classic_pdf(&objects, "")
}

fn encrypted(handler: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut objects = base_objects();
    let dictionary = match handler {
        "Standard" => "<< /Filter /Standard /V 4 /R 4 /Length 128 /O <00112233> /U <44556677> /P -4 /CF << /StdCF << /CFM /AESV2 >> >> /StmF /StdCF /StrF /StdCF /EFF /StdCF >>",
        "Adobe.PubSec" => "<< /Filter /Adobe.PubSec /SubFilter /adbe.pkcs7.s5 /V 4 /Length 128 /Recipients [<00112233>] /CF << /DefaultCryptFilter << /CFM /AESV2 >> >> /StmF /DefaultCryptFilter /StrF /DefaultCryptFilter >>",
        _ => "(not an encryption dictionary)",
    };
    objects.push((6, dictionary));
    classic_pdf(&objects, "/Encrypt 6 0 R")
}

fn metadata_rich() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut objects = base_objects();
    objects[0].1 = "<< /Type /Catalog /Pages 2 0 R /Lang (en-MY) /ViewerPreferences << /DisplayDocTitle true >> /PageMode /UseOutlines /PageLayout /SinglePage /OpenAction [3 0 R /Fit] /Names 7 0 R /Outlines 8 0 R /Metadata 6 0 R /StarPDFUnknown << /KeepMe true >> >>";
    objects.extend([
        (6, "<< /Type /Metadata /Subtype /XML /Length 53 >>\nstream\n<x:xmpmeta xmlns:x=\"adobe:ns:meta/\">v0.11</x:xmpmeta>\nendstream"),
        (7, "<< /Dests << /Names [(start) [3 0 R /Fit]] >> >>"),
        (8, "<< /Type /Outlines /Count 0 >>"),
        (9, "<< /Producer (StarPDF synthetic v0.11) /Title (Metadata preservation) >>"),
    ]);
    classic_pdf(&objects, "/Info 9 0 R")
}

fn ambiguous_orphan_radio() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut objects = base_objects();
    objects[2].1 = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents 4 0 R /Annots [5 0 R 6 0 R] >>";
    objects.extend([
        (5, "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 32768 /T (ambiguous.radio) /Rect [72 620 92 640] /P 3 0 R /V /Off /AS /Off /AP << /N << /Off 7 0 R /A 8 0 R >> >> >>"),
        (6, "<< /Type /Annot /Subtype /Widget /FT /Btn /Ff 32768 /T (ambiguous.radio) /Rect [110 620 130 640] /P 3 0 R /V /Off /AS /Off /AP << /N << /Off 7 0 R /B 8 0 R >> >> >>"),
        (7, "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 0 >>\nstream\n\nendstream"),
        (8, "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 0 >>\nstream\n\nendstream"),
    ]);
    classic_pdf(&objects, "")
}

fn malformed_parent_radio() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut objects = base_objects();
    objects[0].1 = "<< /Type /Catalog /Pages 2 0 R /AcroForm 7 0 R >>";
    objects[2].1 = "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >> /Contents 4 0 R /Annots [6 0 R] >>";
    objects.extend([
        (5, "<< /FT /Btn /Ff 32768 /T (malformed.radio) /Kids [6 0 R] /V /Off >>"),
        (6, "<< /Type /Annot /Subtype /Widget /Parent 99 0 R /Rect [72 620 92 640] /P 3 0 R /AS /Off /AP << /N << /Off 8 0 R /A 9 0 R >> >> >>"),
        (7, "<< /Fields [5 0 R] >>"),
        (8, "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 0 >>\nstream\n\nendstream"),
        (9, "<< /Type /XObject /Subtype /Form /BBox [0 0 20 20] /Length 0 >>\nstream\n\nendstream"),
    ]);
    classic_pdf(&objects, "")
}

fn hybrid_multi_revision() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let catalog = pdf.len();
    pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
    let pages = pdf.len();
    pdf.extend_from_slice(b"2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n");
    let page = pdf.len();
    pdf.extend_from_slice(
        b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>\nendobj\n",
    );
    let stream_offset = pdf.len();
    let data = [1, ((page >> 8) & 0xff) as u8, (page & 0xff) as u8, 0];
    write!(
        &mut pdf,
        "4 0 obj\n<< /Type /XRef /Size 5 /W [1 2 1] /Index [3 1] /Length 4 >>\nstream\n"
    )?;
    pdf.extend_from_slice(&data);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let first_xref = pdf.len();
    write!(&mut pdf, "xref\n0 3\n0000000000 65535 f \n{catalog:010} 00000 n \n{pages:010} 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R /XRefStm {stream_offset} /ID [<0011> <2233>] >>\nstartxref\n{first_xref}\n%%EOF\n")?;
    let replacement = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 700 900] /StarPDFRevision (latest) >>\nendobj\n");
    let second_xref = pdf.len();
    write!(&mut pdf, "xref\n3 1\n{replacement:010} 00000 n \ntrailer\n<< /Size 5 /Root 1 0 R /Prev {first_xref} /ID [<0011> <4455>] >>\nstartxref\n{second_xref}\n%%EOF\n")?;
    Ok(pdf)
}

fn hybrid_signed_encrypted() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut pdf = b"%PDF-1.7\n".to_vec();
    let mut offsets = BTreeMap::new();
    for (number, body) in [
        (1, "<< /Type /Catalog /Pages 2 0 R /AcroForm 7 0 R >>"),
        (2, "<< /Type /Pages /Kids [3 0 R] /Count 1 >>"),
        (3, "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Annots [5 0 R] >>"),
        (5, "<< /Type /Annot /Subtype /Widget /FT /Sig /T (HybridSignature) /Rect [72 620 300 670] /P 3 0 R /V 6 0 R >>"),
        (6, "<< /Type /Sig /Filter /Adobe.PPKLite /SubFilter /adbe.pkcs7.detached /ByteRange [0 64 128 64] /Contents <0011223344556677> >>"),
        (7, "<< /Fields [5 0 R] /SigFlags 3 >>"),
        (8, "<< /Filter /Standard /V 4 /R 4 /Length 128 /O <00112233> /U <44556677> /P -4 /CF << /StdCF << /CFM /AESV2 >> >> /StmF /StdCF /StrF /StdCF >>"),
    ] {
        offsets.insert(number, pdf.len());
        write!(&mut pdf, "{number} 0 obj\n{body}\nendobj\n")?;
    }
    let stream_offset = pdf.len();
    let page_offset = *offsets.get(&3).ok_or("missing page offset")?;
    let page_offset = u32::try_from(page_offset)?;
    write!(
        &mut pdf,
        "9 0 obj\n<< /Type /XRef /Size 10 /W [1 4 2] /Index [3 1] /Length 7 >>\nstream\n"
    )?;
    pdf.extend_from_slice(&[
        1,
        ((page_offset >> 24) & 0xff) as u8,
        ((page_offset >> 16) & 0xff) as u8,
        ((page_offset >> 8) & 0xff) as u8,
        (page_offset & 0xff) as u8,
        0,
        0,
    ]);
    pdf.extend_from_slice(b"\nendstream\nendobj\n");
    let first_xref = pdf.len();
    pdf.extend_from_slice(b"xref\n0 10\n0000000000 65535 f \n");
    for number in 1..10 {
        let offset = if number == 9 {
            Some(stream_offset)
        } else {
            offsets.get(&number).copied()
        };
        if let Some(offset) = offset {
            writeln!(&mut pdf, "{offset:010} 00000 n ")?;
        } else {
            pdf.extend_from_slice(b"0000000000 00000 f \n");
        }
    }
    write!(&mut pdf, "trailer\n<< /Size 10 /Root 1 0 R /Encrypt 8 0 R /XRefStm {stream_offset} /ID [<0011> <2233>] >>\nstartxref\n{first_xref}\n%%EOF\n")?;
    let replacement = pdf.len();
    pdf.extend_from_slice(b"3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 700 900] /Annots [5 0 R] /StarPDFRevision (security-latest) >>\nendobj\n");
    let second_xref = pdf.len();
    write!(&mut pdf, "xref\n3 1\n{replacement:010} 00000 n \ntrailer\n<< /Size 10 /Prev {first_xref} /ID [<0011> <4455>] >>\nstartxref\n{second_xref}\n%%EOF\n")?;
    Ok(pdf)
}

fn main() -> Result<(), Box<dyn Error>> {
    let directory = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tests/fixtures/v0_11_complex"));
    std::fs::create_dir_all(&directory)?;
    let fixtures = [
        ("synthetic-signed-valid.pdf", signed(true)?),
        ("synthetic-signed-malformed.pdf", signed(false)?),
        (
            "synthetic-signature-field-only.pdf",
            signature_field_only()?,
        ),
        ("synthetic-encrypted-standard.pdf", encrypted("Standard")?),
        (
            "synthetic-encrypted-public-key.pdf",
            encrypted("Adobe.PubSec")?,
        ),
        ("synthetic-encrypted-malformed.pdf", encrypted("Malformed")?),
        ("synthetic-metadata-rich.pdf", metadata_rich()?),
        (
            "synthetic-ambiguous-orphan-radio.pdf",
            ambiguous_orphan_radio()?,
        ),
        (
            "synthetic-malformed-parent-radio.pdf",
            malformed_parent_radio()?,
        ),
        (
            "synthetic-hybrid-multi-revision.pdf",
            hybrid_multi_revision()?,
        ),
        (
            "synthetic-hybrid-signed-encrypted.pdf",
            hybrid_signed_encrypted()?,
        ),
    ];
    for (name, bytes) in fixtures {
        std::fs::write(directory.join(name), bytes)?;
    }
    Ok(())
}
