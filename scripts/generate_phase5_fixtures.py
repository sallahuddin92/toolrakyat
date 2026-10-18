#!/usr/bin/env python3
"""Generates canonical Phase 5 test fixtures for advanced graphics semantics:
1. text-clipping-tr.pdf: Text clipping modes (Tr 5 / Tr 7) clipping background vector fills.
2. type3-font-sample.pdf: Type 3 vector charprocs font.
3. tiling-pattern.pdf: PatternType 1 colored tiling pattern.
4. transparency-group-smask.pdf: Form XObject with /Group << /S /Transparency >> and ExtGState transparency.
"""

from pathlib import Path

OUT_DIR = Path("test-assets")
OUT_DIR.mkdir(parents=True, exist_ok=True)


def create_pdf(objects, root_ref, info_ref=None):
    """Assembles a valid PDF from a list of (obj_num, obj_str) tuples."""
    out = bytearray(b"%PDF-1.7\n%\x80\x81\x82\x83\n")
    offsets = {}
    for num, body in sorted(objects, key=lambda x: x[0]):
        offsets[num] = len(out)
        out.extend(f"{num} 0 obj\n{body}\nendobj\n".encode("latin1"))

    startxref = len(out)
    max_obj = max(offsets.keys()) + 1
    out.extend(f"xref\n0 {max_obj}\n0000000000 65535 f \n".encode("latin1"))
    for i in range(1, max_obj):
        if i in offsets:
            out.extend(f"{offsets[i]:010d} 00000 n \n".encode("latin1"))
        else:
            out.extend(b"0000000000 00001 f \n")

    info_part = f" /Info {info_ref} 0 R" if info_ref else ""
    out.extend(
        f"trailer\n<< /Size {max_obj} /Root {root_ref} 0 R{info_part} >>\nstartxref\n{startxref}\n%%EOF\n".encode(
            "latin1"
        )
    )
    return bytes(out)


def build_text_clipping_fixture():
    # Page with Text Rendering Mode 5 (Stroke + Clip) clipping a colorful radial-like grid
    stream = """
q
1 1 1 rg 0 0 612 792 re f
0 0 0 RG 2 w
BT
/F1 48 Tf
5 Tr
50 700 Td
(CLIPPED TEXT) Tj
ET
% Now paint rectangles that will be clipped by the text outlines
1 0 0 rg 50 680 400 60 re f
0 0 1 rg 50 680 200 60 re f
0 1 0 rg 150 680 100 60 re f
Q
q
0 0 0 rg
BT
/F1 16 Tf
0 Tr
50 640 Td
(Regular text below text clipping boundary) Tj
ET
Q
""".strip()

    objs = [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>"),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 612 792 ] /Contents 4 0 R /Resources 5 0 R >>",
        ),
        (4, f"<< /Length {len(stream)} >>\nstream\n{stream}\nendstream"),
        (
            5,
            "<< /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold >> >> >>",
        ),
    ]
    return create_pdf(objs, 1)


def build_type3_font_fixture():
    # Type 3 font with 'A' (Box) and 'B' (Triangle)
    charproc_a = "1000 0 d0 100 100 800 800 re f".strip()
    charproc_b = "1000 0 d0 100 100 m 500 900 l 900 100 l h f".strip()

    stream = """
q
1 1 1 rg 0 0 612 792 re f
0.2 0.4 0.8 rg
BT
/T3 36 Tf
50 700 Td
(ABAB) Tj
ET
0 0 0 rg
BT
/F1 16 Tf
50 640 Td
(Type 3 font shapes: A=Square, B=Triangle) Tj
ET
Q
""".strip()

    objs = [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>"),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 612 792 ] /Contents 4 0 R /Resources 5 0 R >>",
        ),
        (4, f"<< /Length {len(stream)} >>\nstream\n{stream}\nendstream"),
        (5, "<< /Font << /T3 6 0 R /F1 7 0 R >> >>"),
        (
            6,
            "<< /Type /Font /Subtype /Type3 /FontBBox [ 0 0 1000 1000 ] /FontMatrix [ 0.001 0 0 0.001 0 0 ] /FirstChar 65 /LastChar 66 /Widths [ 1000 1000 ] /Encoding << /Type /Encoding /Differences [ 65 /A /B ] >> /CharProcs << /A 8 0 R /B 9 0 R >> >>",
        ),
        (7, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
        (8, f"<< /Length {len(charproc_a)} >>\nstream\n{charproc_a}\nendstream"),
        (9, f"<< /Length {len(charproc_b)} >>\nstream\n{charproc_b}\nendstream"),
    ]
    return create_pdf(objs, 1)


def build_tiling_pattern_fixture():
    # PatternType 1 colored pattern: 20x20 tile with two colored squares
    pat_stream = "1 0.2 0.2 rg 0 0 10 10 re f 0.2 0.2 1 rg 10 10 10 10 re f".strip()

    stream = """
q
1 1 1 rg 0 0 612 792 re f
/Cs1 cs
/P1 scn
50 500 300 200 re f
0 0 0 rg
BT
/F1 16 Tf
50 460 Td
(Tiling Pattern /PatternType 1 Fill Demo) Tj
ET
Q
""".strip()

    objs = [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>"),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 612 792 ] /Contents 4 0 R /Resources 5 0 R >>",
        ),
        (4, f"<< /Length {len(stream)} >>\nstream\n{stream}\nendstream"),
        (
            5,
            "<< /ColorSpace << /Cs1 [ /Pattern /DeviceRGB ] >> /Pattern << /P1 6 0 R >> /Font << /F1 7 0 R >> >>",
        ),
        (
            6,
            f"<< /Type /Pattern /PatternType 1 /PaintType 1 /TilingType 1 /BBox [ 0 0 20 20 ] /XStep 20 /YStep 20 /Resources << >> /Length {len(pat_stream)} >>\nstream\n{pat_stream}\nendstream",
        ),
        (7, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
    ]
    return create_pdf(objs, 1)


def build_transparency_group_fixture():
    # Transparency group Form XObject with overlapping semi-transparent circles
    form_stream = """
q
/GS1 gs
1 0 0 rg
0 0 150 150 re f
0 1 0 rg
50 50 150 150 re f
Q
""".strip()

    page_stream = """
q
1 1 1 rg 0 0 612 792 re f
0 0 1 rg
100 450 200 200 re f
q
1 0 0 1 150 500 cm
/Fm1 Do
Q
0 0 0 rg
BT
/F1 16 Tf
50 400 Td
(Isolated Transparency Group Form XObject with alpha blending) Tj
ET
Q
""".strip()

    objs = [
        (1, "<< /Type /Catalog /Pages 2 0 R >>"),
        (2, "<< /Type /Pages /Kids [ 3 0 R ] /Count 1 >>"),
        (
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [ 0 0 612 792 ] /Contents 4 0 R /Resources 5 0 R /Group 9 0 R >>",
        ),
        (4, f"<< /Length {len(page_stream)} >>\nstream\n{page_stream}\nendstream"),
        (
            5,
            "<< /XObject << /Fm1 6 0 R >> /Font << /F1 8 0 R >> >>",
        ),
        (
            6,
            f"<< /Type /XObject /Subtype /Form /FormType 1 /BBox [ 0 0 200 200 ] /Group << /Type /Group /S /Transparency /CS /DeviceRGB >> /Resources << /ExtGState << /GS1 7 0 R >> >> /Length {len(form_stream)} >>\nstream\n{form_stream}\nendstream",
        ),
        (7, "<< /Type /ExtGState /ca 0.6 /CA 0.6 /BM /Normal >>"),
        (8, "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>"),
        (9, "<< /Type /Group /S /Transparency /CS /DeviceRGB >>"),
    ]
    return create_pdf(objs, 1)


def main():
    fixtures = {
        "text-clipping-tr.pdf": build_text_clipping_fixture(),
        "type3-font-sample.pdf": build_type3_font_fixture(),
        "tiling-pattern.pdf": build_tiling_pattern_fixture(),
        "transparency-group-smask.pdf": build_transparency_group_fixture(),
    }

    for name, data in fixtures.items():
        path = OUT_DIR / name
        path.write_bytes(data)
        print(f"Generated {path} ({len(data)} bytes)")


if __name__ == "__main__":
    main()
