#!/usr/bin/env python3
"""Phase 8 Real-World Corpus Generator.

Generates 10 diverse, high-fidelity real-world PDF documents to expand the StarPDF
qualification corpus beyond the 52 baseline fixtures:
1. office-invoice-enterprise.pdf (Office / Financial invoice)
2. office-technical-report.pdf (Office / 3-page multi-column engineering report)
3. scanned-multires-archive.pdf (Scanned historical archive with mixed image layers)
4. multilingual-arabic-jawi.pdf (RTL Arabic and Jawi typography with embedded Noto font)
5. multilingual-cjk-unicode.pdf (Mixed CJK with embedded Noto font)
6. multilingual-hebrew-bidi.pdf (RTL Hebrew with bidirectional flow and embedded Noto font)
7. forms-employment-application.pdf (Interactive AcroForm application with widgets)
8. annotations-markup-review.pdf (Rich document annotations: highlight, freetext, ink, square)
9. large-manual-20page.pdf (Large-document handling: 20-page technical manual)
10. complex-infographic-graphics.pdf (Complex vector paths, bezier curves, transparency)
"""

import io
import math
import os
from pathlib import Path
import fitz  # PyMuPDF
from PIL import Image, ImageDraw

CORPUS_DIR = Path("test-assets/realworld_corpus")
CORPUS_DIR.mkdir(parents=True, exist_ok=True)

FONT_ARABIC = "apps/smartpdf-web/public/fonts/NotoSansArabic-Regular.ttf"
FONT_HEBREW = "apps/smartpdf-web/public/fonts/NotoSansHebrew-Regular.ttf"
FONT_CJK = "apps/smartpdf-web/public/fonts/NotoSansSC-Regular.ttf"

def save_optimized(doc, path):
    try:
        doc.subset_fonts()
    except Exception:
        pass
    doc.save(str(path), garbage=4, deflate=True)

print(f"Generating Phase 8 Real-World Corpus in {CORPUS_DIR}...")

# ----------------------------------------------------------------------
# 1. office-invoice-enterprise.pdf
# ----------------------------------------------------------------------
def gen_invoice():
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)  # Letter
    
    # Header & Logo bar
    shape = page.new_shape()
    shape.draw_rect(fitz.Rect(40, 40, 572, 90))
    shape.finish(color=None, fill=(0.06, 0.09, 0.16))  # Slate 900
    shape.commit()
    
    page.insert_text((55, 72), "TOOLRAKYAT ENTERPRISE CLOUD", fontsize=18, color=(1, 1, 1), fontname="helv")
    page.insert_text((440, 72), "INVOICE #INV-2026-904", fontsize=11, color=(0.7, 0.8, 1.0), fontname="helv")
    
    # Metadata
    page.insert_text((40, 120), "Billed To: Acronis Logistics Sdn Bhd", fontsize=11, fontname="helv")
    page.insert_text((40, 135), "Tax ID: MY-8829104-X | Currency: USD ($)", fontsize=9, color=(0.3, 0.3, 0.3), fontname="helv")
    page.insert_text((420, 120), "Issue Date: October 19, 2026", fontsize=10, fontname="helv")
    page.insert_text((420, 135), "Payment Due: Net 30 Days", fontsize=10, fontname="helv")
    
    # Table Header
    shape = page.new_shape()
    shape.draw_rect(fitz.Rect(40, 160, 572, 185))
    shape.finish(color=(0.8, 0.8, 0.8), fill=(0.95, 0.96, 0.98), width=1)
    shape.commit()
    
    page.insert_text((50, 177), "Line Item / Description", fontsize=10, fontname="helv")
    page.insert_text((340, 177), "Qty", fontsize=10, fontname="helv")
    page.insert_text((410, 177), "Rate", fontsize=10, fontname="helv")
    page.insert_text((500, 177), "Amount", fontsize=10, fontname="helv")
    
    # Items
    items = [
        ("StarPDF Native GPU Engine Enterprise License (Per Seat)", 25, 450.00),
        ("ToolRakyat Core Cloud API Ingestion Tier - Q4", 1, 3500.00),
        ("Dedicated Document Infrastructure Support Retainer", 3, 1200.00),
        ("Offline Slint GPU Acceleration Desktop Runtime Bundle", 25, 120.00),
        ("Compliance & Security Export Audit Suite (PDF/A-2b)", 1, 2800.00),
    ]
    
    y = 205
    subtotal = 0.0
    for name, qty, rate in items:
        amt = qty * rate
        subtotal += amt
        page.insert_text((50, y), name, fontsize=9, fontname="helv")
        page.insert_text((345, y), str(qty), fontsize=9, fontname="helv")
        page.insert_text((410, y), f"${rate:,.2f}", fontsize=9, fontname="helv")
        page.insert_text((500, y), f"${amt:,.2f}", fontsize=9, fontname="helv")
        
        # Grid line
        shape = page.new_shape()
        shape.draw_line(fitz.Point(40, y + 6), fitz.Point(572, y + 6))
        shape.finish(color=(0.9, 0.9, 0.9), width=0.5)
        shape.commit()
        y += 26
        
    # Totals
    y += 15
    tax = subtotal * 0.08
    total = subtotal + tax
    page.insert_text((400, y), f"Subtotal:       ${subtotal:,.2f}", fontsize=10, fontname="helv")
    page.insert_text((400, y + 18), f"Sales Tax (8%): ${tax:,.2f}", fontsize=10, fontname="helv")
    
    shape = page.new_shape()
    shape.draw_rect(fitz.Rect(390, y + 30, 572, y + 60))
    shape.finish(color=(0.2, 0.4, 0.8), fill=(0.92, 0.95, 1.0), width=1.5)
    shape.commit()
    page.insert_text((400, y + 51), f"TOTAL DUE:    ${total:,.2f}", fontsize=12, color=(0.05, 0.2, 0.6), fontname="helv")
    
    # Footer
    page.insert_text((40, 750), "Thank you for your business. Electronic funds transfer details enclosed on reverse.", fontsize=8, color=(0.5, 0.5, 0.5), fontname="helv")
    
    save_optimized(doc, CORPUS_DIR / "office-invoice-enterprise.pdf")
    print("  -> Generated office-invoice-enterprise.pdf (1 page)")

# ----------------------------------------------------------------------
# 2. office-technical-report.pdf (3 pages)
# ----------------------------------------------------------------------
def gen_report():
    doc = fitz.open()
    for p_idx in range(3):
        page = doc.new_page(width=612, height=792)
        # Running header
        shape = page.new_shape()
        shape.draw_line(fitz.Point(40, 45), fitz.Point(572, 45))
        shape.finish(color=(0.7, 0.7, 0.7), width=0.75)
        shape.commit()
        page.insert_text((40, 40), "STARPDF ARCHITECTURE SPECIFICATION v0.24", fontsize=8, color=(0.4, 0.4, 0.4), fontname="helv")
        page.insert_text((510, 40), f"PAGE {p_idx+1} OF 3", fontsize=8, color=(0.4, 0.4, 0.4), fontname="helv")
        
        if p_idx == 0:
            page.insert_text((40, 85), "Native GPU-Rasterized PDF Pipeline", fontsize=20, fontname="helv")
            page.insert_text((40, 110), "Section 1: Abstract and System Foundations", fontsize=12, color=(0.3, 0.3, 0.6), fontname="helv")
            body = (
                "The StarPDF GPU rasterizer provides hardware-accelerated rendering directly from the "
                "pure-Rust StarPDF Render IR. Prior implementations relied on CPU rasterization via tiny-skia, "
                "incurring significant CPU overhead on complex vector shapes and dense text layouts. "
                "Phase 7 and Phase 8 introduce native TrueType/OpenType glyph outline tessellation and strict "
                "sequential z-order compositing without a global CPU overlay."
            )
            rect = fitz.Rect(40, 130, 572, 220)
            page.insert_textbox(rect, body, fontsize=10, fontname="helv")
            
            # Diagram box
            shape = page.new_shape()
            shape.draw_rect(fitz.Rect(40, 240, 572, 420))
            shape.finish(color=(0.3, 0.5, 0.8), fill=(0.96, 0.98, 1.0), width=1)
            shape.commit()
            page.insert_text((60, 270), "[Figure 1: Dual-Phase Command Execution Architecture]", fontsize=11, fontname="helv")
            page.insert_text((60, 300), "Phase 1: Parse Render IR -> Tessellate Lyon paths & Glyphs -> Pre-allocate Buffers", fontsize=9, fontname="courier")
            page.insert_text((60, 325), "Phase 2: Begin wgpu::RenderPass -> Sequential GpuDrawCmd Execution -> Submit", fontsize=9, fontname="courier")
            page.insert_text((60, 350), "Clipping: Hardware Scissor Rectangles + Scoped CPU Transparency Fallbacks", fontsize=9, fontname="courier")
            page.insert_text((60, 375), "Performance: >15x speedup over CPU on text-heavy mixed pages", fontsize=9, fontname="courier")
            
            # Two column text below
            col1 = "The compilation phase maps PDF content streams into typed RenderItem representations. Text runs contain codepoints, matrix transformations, and font references. Vector paths contain cubic and quadratic beziers."
            col2 = "All vertex coordinates undergo scale and translation transformations directly mapped to device pixels. Hardware scissor rectangles isolate clipping regions without requiring multi-pass stencil buffers."
            page.insert_textbox(fitz.Rect(40, 440, 290, 600), col1, fontsize=9, fontname="helv")
            page.insert_textbox(fitz.Rect(310, 440, 560, 600), col2, fontsize=9, fontname="helv")
            
        elif p_idx == 1:
            page.insert_text((40, 85), "Section 2: Multi-Column Memory and Layout Profiling", fontsize=16, fontname="helv")
            # Table
            shape = page.new_shape()
            shape.draw_rect(fitz.Rect(40, 110, 572, 280))
            shape.finish(color=(0.8, 0.8, 0.8), width=1)
            for y_line in [140, 175, 210, 245]:
                shape.draw_line(fitz.Point(40, y_line), fitz.Point(572, y_line))
                shape.finish(color=(0.85, 0.85, 0.85), width=0.5)
            shape.commit()
            
            page.insert_text((50, 130), "Benchmark Target", fontsize=10, fontname="helv")
            page.insert_text((220, 130), "CPU tiny-skia (ms)", fontsize=10, fontname="helv")
            page.insert_text((360, 130), "GPU Render (ms)", fontsize=10, fontname="helv")
            page.insert_text((480, 130), "Pass Speedup", fontsize=10, fontname="helv")
            
            rows = [
                ("text-multicol-table.pdf", "49.46 ms", "2.22 ms", "22.28x"),
                ("text-multispan-heading.pdf", "48.14 ms", "3.00 ms", "16.05x"),
                ("scanned-test.pdf", "323.22 ms", "1.96 ms", "164.72x"),
                ("smartpdf-adobe-like-form.pdf", "28.35 ms", "2.56 ms", "11.09x"),
            ]
            r_y = 160
            for target, cpu, gpu, spd in rows:
                page.insert_text((50, r_y), target, fontsize=9, fontname="courier")
                page.insert_text((230, r_y), cpu, fontsize=9, fontname="helv")
                page.insert_text((370, r_y), gpu, fontsize=9, fontname="helv")
                page.insert_text((490, r_y), spd, fontsize=9, fontname="helv")
                r_y += 35
                
            page.insert_textbox(fitz.Rect(40, 310, 572, 600),
                "The table above underscores the empirical performance advantages observed on Apple Silicon Metal targets. "
                "GPU execution demonstrates near constant-time rasterization across complex glyph metrics. "
                "Memory allocations remain bounded under 250 MB even during rapid zoom sweeps and tab transitions.",
                fontsize=9.5, fontname="helv")
                
        else:
            page.insert_text((40, 85), "Section 3: Qualification Matrix & Conclusions", fontsize=16, fontname="helv")
            page.insert_textbox(fitz.Rect(40, 110, 572, 300),
                "Real-world qualification requires robust handling of non-standard fonts, broken xref tables, "
                "and legacy PDF operators. By preserving fail-safe CPU fallbacks for unsupported shading models "
                "while delegating 100% of standard vector paths, text glyphs, and images to the GPU, StarPDF delivers "
                "production stability with zero silent degradation.",
                fontsize=10, fontname="helv")
            
            # Sign-off box
            shape = page.new_shape()
            shape.draw_rect(fitz.Rect(40, 320, 572, 420))
            shape.finish(color=(0.2, 0.6, 0.4), fill=(0.95, 0.99, 0.96), width=1)
            shape.commit()
            page.insert_text((60, 350), "VERIFIED BY: ToolRakyat Desktop Engineering Team", fontsize=11, fontname="helv")
            page.insert_text((60, 375), "SIGNATURE STAMP: [ SHA-256: 0a9e882b48d... VALIDATED ]", fontsize=9, fontname="courier")
            page.insert_text((60, 400), "STATUS: QUALIFIED FOR PRODUCTION RELEASE", fontsize=10, color=(0.1, 0.5, 0.2), fontname="helv")
            
        # Running footer
        page.insert_text((40, 760), "CONFIDENTIAL — TOOLRAKYAT PROPRIETARY SPECIFICATION", fontsize=8, color=(0.6, 0.6, 0.6), fontname="helv")
        
    save_optimized(doc, CORPUS_DIR / "office-technical-report.pdf")
    print("  -> Generated office-technical-report.pdf (3 pages)")

# ----------------------------------------------------------------------
# 3. scanned-multires-archive.pdf
# ----------------------------------------------------------------------
def gen_scan():
    doc = fitz.open()
    page = doc.new_page(width=595, height=842)  # A4
    
    # Create realistic scanned document image with PIL
    w, h = 800, 1131
    img = Image.new("RGB", (w, h), (247, 245, 238))  # Sepia paper tone
    draw = ImageDraw.Draw(img)
    
    # Add scanner noise / paper texture
    import random
    random.seed(42)
    for _ in range(5000):
        rx = random.randint(0, w-1)
        ry = random.randint(0, h-1)
        draw.point((rx, ry), fill=(random.randint(210, 240), random.randint(205, 235), random.randint(195, 225)))
        
    # Draw simulated scanned archival document
    draw.rectangle([60, 60, 740, 120], outline=(80, 75, 70), width=3)
    draw.text((80, 80), "NATIONAL ARCHIVES HISTORICAL DISPATCH #1948", fill=(40, 35, 30))
    draw.line([60, 140, 740, 140], fill=(80, 75, 70), width=2)
    
    # Simulated typewriter paragraphs
    lines = [
        "MEMORANDUM FOR THE CHIEF OF STRATEGIC SERVICES",
        "SUBJECT: Evaluation of High-Resolution Multi-Tier Document Compression",
        "",
        "1. Archival reproduction requires preserving authentic scan artifacts while",
        "ensuring sub-millisecond GPU upload and rendering times.",
        "2. The scanned image stream contains uncompressed RGB samples alongside",
        "vector stamps and historical hand-written accession stamps.",
        "3. Recent testing confirms that StarPDF GPU image sampling achieves >100x",
        "speedups compared to traditional CPU memory rasterization pipelines.",
        "",
        "ACCESSION STATUS: UNCLASSIFIED / PUBLIC DOMAIN",
    ]
    ty = 170
    for l in lines:
        draw.text((80, ty), l, fill=(50, 45, 40))
        ty += 30
        
    # Red Confidential/Accession Stamp
    draw.rectangle([480, 800, 720, 890], outline=(180, 40, 30), width=4)
    draw.text((500, 830), "DECLASSIFIED", fill=(180, 40, 30))
    draw.text((500, 855), "OCTOBER 2026", fill=(180, 40, 30))
    
    buf = io.BytesIO()
    img.save(buf, format="JPEG", quality=85)
    img_bytes = buf.getvalue()
    
    # Insert image into PDF
    page.insert_image(fitz.Rect(0, 0, 595, 842), stream=img_bytes)
    
    # Overlay real vector stamp on top to test mixed image/vector z-order
    shape = page.new_shape()
    shape.draw_rect(fitz.Rect(40, 700, 280, 760))
    shape.finish(color=(0.1, 0.4, 0.8), fill=(0.9, 0.95, 1.0), width=2)
    shape.commit()
    page.insert_text((55, 735), "VERIFIED ARCHIVAL OCR LAYER", fontsize=11, color=(0.1, 0.3, 0.7), fontname="helv")
    
    save_optimized(doc, CORPUS_DIR / "scanned-multires-archive.pdf")
    print("  -> Generated scanned-multires-archive.pdf (1 page)")

# ----------------------------------------------------------------------
# 4. multilingual-arabic-jawi.pdf
# ----------------------------------------------------------------------
def gen_arabic_jawi():
    doc = fitz.open()
    page = doc.new_page(width=595, height=842)
    page.insert_font(fontname="arabic", fontfile=FONT_ARABIC)
    
    # Title
    page.insert_text((40, 60), "Multilingual Typography Test: Arabic & Jawi Script", fontsize=16, fontname="helv")
    page.insert_text((40, 80), "Verifying Right-to-Left (RTL), Ligatures, and Fallback Font Tessellation", fontsize=10, color=(0.4, 0.4, 0.4), fontname="helv")
    
    # Background card
    shape = page.new_shape()
    shape.draw_rect(fitz.Rect(40, 110, 555, 340))
    shape.finish(color=(0.2, 0.5, 0.4), fill=(0.96, 0.99, 0.97), width=1)
    shape.commit()
    
    arabic_samples = [
        ("العربية (Standard Arabic)", "بسم الله الرحمن الرحيم - السلام عليكم ورحمة الله وبركاته"),
        ("الأمم المتحدة (UN Declaration)", "يولد جميع الناس أحراراً ومتساوين في الكرامة والحقوق"),
        ("فاطمة ڤيکتوريا (Jawi / Malay)", "بهاس ملايو توليسن جاوي - سلامت داتڠ کمنارا بارو"),
        ("العلم والعمل (Proverb)", "العلم في الصغر كالنقش على الحجر"),
    ]
    
    y = 145
    for title, text in arabic_samples:
        page.insert_text((60, y), title, fontsize=11, color=(0.1, 0.3, 0.2), fontname="helv")
        page.insert_text((60, y + 22), text, fontname="arabic", fontsize=14)
        y += 48
        
    page.insert_textbox(fitz.Rect(40, 360, 555, 520),
        "In StarPDF Phase 7 & 8, glyphs missing from embedded document font dictionaries are resolved "
        "through the FallbackFontCache (NotoSansArabic-Regular). TrueType outline tables are dynamically "
        "tessellated on the GPU via skrifa and LyonPen, ensuring exact visual parity and RTL positioning.",
        fontsize=10, fontname="helv")
        
    save_optimized(doc, CORPUS_DIR / "multilingual-arabic-jawi.pdf")
    print("  -> Generated multilingual-arabic-jawi.pdf (1 page)")

# ----------------------------------------------------------------------
# 5. multilingual-cjk-unicode.pdf
# ----------------------------------------------------------------------
def gen_cjk():
    doc = fitz.open()
    page = doc.new_page(width=595, height=842)
    page.insert_font(fontname="cjk", fontfile=FONT_CJK)
    
    page.insert_text((40, 60), "CJK Multilingual Typography Verification", fontsize=16, fontname="helv")
    page.insert_text((40, 80), "Simplified Chinese, Traditional Chinese, Japanese Kanji/Kana, Korean Hangul", fontsize=10, color=(0.4, 0.4, 0.4), fontname="helv")
    
    shape = page.new_shape()
    shape.draw_rect(fitz.Rect(40, 110, 555, 380))
    shape.finish(color=(0.3, 0.3, 0.7), fill=(0.97, 0.97, 1.0), width=1)
    shape.commit()
    
    cjk_samples = [
        ("Simplified Chinese (简体中文)", "万物皆有裂痕，那是光照进来的地方。开源软件改变世界。"),
        ("Traditional Chinese (繁體中文)", "天地有正氣，雜然賦流形。下則為河嶽，上則為日星。"),
        ("Japanese (日本語)", "吾輩は猫である。名前はまだ無い。どこで生れたかとんと見当がつかぬ。"),
        ("Korean Hangul (한국어)", "나랏말싸미 듕귁에 달아 문자와로 서르 사맛디 아니할쌔."),
    ]
    
    y = 150
    for title, text in cjk_samples:
        page.insert_text((60, y), title, fontsize=11, color=(0.2, 0.2, 0.5), fontname="helv")
        page.insert_text((60, y + 22), text, fontname="cjk", fontsize=11)
        y += 55
        
    page.insert_textbox(fitz.Rect(40, 410, 555, 600),
        "CJK characters contain high-density stroke vectors. StarPDF's GPU tessellator converts "
        "Type0 and CIDFont outlines directly into triangle index buffers without CPU intermediate pixmaps, "
        "enabling instantaneous zoom and multi-resolution fidelity across Asian language corpora.",
        fontsize=10, fontname="helv")
        
    save_optimized(doc, CORPUS_DIR / "multilingual-cjk-unicode.pdf")
    print("  -> Generated multilingual-cjk-unicode.pdf (1 page)")

# ----------------------------------------------------------------------
# 6. multilingual-hebrew-bidi.pdf
# ----------------------------------------------------------------------
def gen_hebrew_bidi():
    doc = fitz.open()
    page = doc.new_page(width=595, height=842)
    page.insert_font(fontname="hebrew", fontfile=FONT_HEBREW)
    
    page.insert_text((40, 60), "Hebrew Bidirectional (BiDi) Layout Verification", fontsize=16, fontname="helv")
    page.insert_text((40, 80), "Validating Hebrew Script, Niqqud diacritics, and Embedded LTR Numerical Data", fontsize=10, color=(0.4, 0.4, 0.4), fontname="helv")
    
    shape = page.new_shape()
    shape.draw_rect(fitz.Rect(40, 110, 555, 320))
    shape.finish(color=(0.6, 0.3, 0.2), fill=(1.0, 0.98, 0.96), width=1)
    shape.commit()
    
    samples = [
        ("Modern Hebrew (עִבְרִית מודרנית)", "כל בני האדם נולדו בני חורין ושווים בערכם ובזכויותיהם."),
        ("BiDi Mixed with LTR Numbers", "המחיר הכולל עבור 25 יחידות הוא $4,500.00 בלבד."),
        ("Technical Terminology (מונחים טכניים)", "עיבוד גרפי באמצעות GPU מאיץ את ביצועי StarPDF ב-1500%."),
    ]
    
    y = 150
    for title, text in samples:
        page.insert_text((60, y), title, fontsize=11, color=(0.4, 0.2, 0.1), fontname="helv")
        page.insert_text((60, y + 22), text, fontname="hebrew", fontsize=13)
        y += 50
        
    save_optimized(doc, CORPUS_DIR / "multilingual-hebrew-bidi.pdf")
    print("  -> Generated multilingual-hebrew-bidi.pdf (1 page)")

# ----------------------------------------------------------------------
# 7. forms-employment-application.pdf
# ----------------------------------------------------------------------
def gen_form():
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)
    
    # Title
    page.insert_text((40, 55), "APPLICATION FOR EMPLOYMENT — SMARTPDF LABS", fontsize=15, fontname="helv")
    page.insert_text((40, 72), "Please complete all interactive fields below. Forms comply with ISO 32000-1 AcroForm standards.", fontsize=8.5, color=(0.4, 0.4, 0.4), fontname="helv")
    
    # Section 1: Personal Info
    shape = page.new_shape()
    shape.draw_rect(fitz.Rect(40, 90, 572, 115))
    shape.finish(color=None, fill=(0.15, 0.25, 0.4))
    shape.commit()
    page.insert_text((50, 107), "1. APPLICANT IDENTIFICATION", fontsize=10, color=(1, 1, 1), fontname="helv")
    
    # Text Fields
    fields = [
        ("Full Legal Name:", fitz.Rect(40, 140, 300, 165), "name_field", "Jane Doe, M.Sc."),
        ("National ID / Passport:", fitz.Rect(320, 140, 572, 165), "id_field", "A-9920148-X"),
        ("Email Address:", fitz.Rect(40, 190, 300, 215), "email_field", "jane.doe@example.com"),
        ("Phone Number:", fitz.Rect(320, 190, 572, 215), "phone_field", "+60 12-345-6789"),
    ]
    
    for label, r, fname, default_val in fields:
        page.insert_text((r.x0, r.y0 - 5), label, fontsize=8.5, fontname="helv")
        widget = fitz.Widget()
        widget.rect = r
        widget.field_type = fitz.PDF_WIDGET_TYPE_TEXT
        widget.field_name = fname
        widget.field_value = default_val
        widget.text_fontsize = 9
        widget.border_color = (0.7, 0.7, 0.7)
        widget.fill_color = (0.97, 0.98, 1.0)
        page.add_widget(widget)
        
    # Section 2: Preferences & Checkboxes
    shape = page.new_shape()
    shape.draw_rect(fitz.Rect(40, 240, 572, 265))
    shape.finish(color=None, fill=(0.15, 0.25, 0.4))
    shape.commit()
    page.insert_text((50, 257), "2. ELIGIBILITY & WORK AUTHORIZATION", fontsize=10, color=(1, 1, 1), fontname="helv")
    
    checkboxes = [
        ("Authorized to work in country of application without sponsorship", fitz.Rect(45, 280, 60, 295), "auth_cb", True),
        ("Possess minimum 3 years of Rust systems programming experience", fitz.Rect(45, 310, 60, 325), "rust_cb", True),
        ("Comfortable with Metal/Vulkan GPU shader pipelines and Lyon tessellation", fitz.Rect(45, 340, 60, 355), "gpu_cb", True),
    ]
    for label, r, cbname, val in checkboxes:
        widget = fitz.Widget()
        widget.rect = r
        widget.field_type = fitz.PDF_WIDGET_TYPE_CHECKBOX
        widget.field_name = cbname
        widget.field_value = "Yes" if val else "Off"
        widget.border_color = (0.3, 0.3, 0.3)
        page.add_widget(widget)
        page.insert_text((r.x1 + 10, r.y0 + 11), label, fontsize=9, fontname="helv")
        
    # Signature Box
    shape = page.new_shape()
    shape.draw_rect(fitz.Rect(40, 400, 572, 480))
    shape.finish(color=(0.5, 0.5, 0.5), fill=(0.99, 0.99, 0.99), width=1)
    shape.commit()
    page.insert_text((50, 420), "APPLICANT ELECTRONIC SIGNATURE:", fontsize=9, fontname="helv")
    page.insert_text((50, 455), "Jane Doe (Digitally Signed via StarPDF Security Session)", fontsize=12, color=(0.1, 0.2, 0.5), fontname="courier")
    page.insert_text((420, 455), "Date: 2026-10-19", fontsize=9, fontname="helv")
    
    save_optimized(doc, CORPUS_DIR / "forms-employment-application.pdf")
    print("  -> Generated forms-employment-application.pdf (1 page)")

# ----------------------------------------------------------------------
# 8. annotations-markup-review.pdf
# ----------------------------------------------------------------------
def gen_annotations():
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)
    
    page.insert_text((40, 60), "CONTRACT AGREEMENT: LEGAL REVIEW & REDACTIONS", fontsize=16, fontname="helv")
    page.insert_text((40, 80), "Document markup includes Highlight, Underline, FreeText, Ink, and Square Redaction", fontsize=10, color=(0.4, 0.4, 0.4), fontname="helv")
    
    contract_p1 = (
        "1. DEFINITION OF LICENSED INTELLECTUAL PROPERTY: The licensor hereby grants the licensee an exclusive, "
        "worldwide, royalty-free license to utilize the GPU-accelerated PDF rasterization subsystem for all commercial "
        "and non-commercial desktop and mobile deployments."
    )
    rect1 = fitz.Rect(40, 110, 572, 170)
    page.insert_textbox(rect1, contract_p1, fontsize=10.5, fontname="helv")
    
    # 1. Highlight annotation over key phrase
    annot_hl = page.add_highlight_annot(fitz.Rect(40, 125, 300, 140))
    annot_hl.set_colors(stroke=(1.0, 0.9, 0.2))  # Yellow highlight
    annot_hl.update()
    
    contract_p2 = (
        "2. LIABILITY AND WARRANTY EXCLUSION: In no event shall ToolRakyat or its developers be liable for any direct, "
        "indirect, incidental, or consequential damages arising from high-throughput document rendering operations."
    )
    rect2 = fitz.Rect(40, 185, 572, 240)
    page.insert_textbox(rect2, contract_p2, fontsize=10.5, fontname="helv")
    
    # 2. Underline annotation
    annot_ul = page.add_underline_annot(fitz.Rect(40, 205, 572, 220))
    annot_ul.set_colors(stroke=(0.8, 0.2, 0.2))  # Red underline
    annot_ul.update()
    
    # 3. FreeText Callout annotation
    annot_ft = page.add_freetext_annot(
        fitz.Rect(350, 270, 572, 340),
        "LEGAL COUNSEL NOTE:\nClause 2 verified compliant with\ncommercial indemnity standards.",
        fontsize=9,
        text_color=(0.1, 0.3, 0.1),
        fill_color=(0.92, 0.98, 0.92),
    )
    annot_ft.set_border(width=1)
    annot_ft.update()
    
    # 4. Square Redaction Box (Blackout)
    shape = page.new_shape()
    shape.draw_rect(fitz.Rect(40, 280, 260, 315))
    shape.finish(color=None, fill=(0.05, 0.05, 0.05))  # Black redaction fill
    shape.commit()
    page.insert_text((50, 302), "[CONFIDENTIAL TRADE SECRET REDACTED]", fontsize=8, color=(1, 1, 1), fontname="helv")
    
    # 5. Ink Signature
    annot_ink = page.add_ink_annot([
        [(60.0, 420.0), (80.0, 400.0), (110.0, 435.0), (140.0, 395.0), (180.0, 430.0)]
    ])
    annot_ink.set_colors(stroke=(0.0, 0.2, 0.7))
    annot_ink.set_border(width=2.5)
    annot_ink.update()
    page.insert_text((60, 445), "Authorized Signatory — Counsel of Record", fontsize=8.5, color=(0.4, 0.4, 0.4), fontname="helv")
    
    save_optimized(doc, CORPUS_DIR / "annotations-markup-review.pdf")
    print("  -> Generated annotations-markup-review.pdf (1 page)")

# ----------------------------------------------------------------------
# 9. large-manual-20page.pdf (20 pages)
# ----------------------------------------------------------------------
def gen_large_manual():
    doc = fitz.open()
    for p_idx in range(20):
        page = doc.new_page(width=612, height=792)
        # Header
        shape = page.new_shape()
        shape.draw_line(fitz.Point(40, 45), fitz.Point(572, 45))
        shape.finish(color=(0.6, 0.6, 0.6), width=0.5)
        shape.commit()
        page.insert_text((40, 40), "SMARTPDF COMPREHENSIVE USER MANUAL — REVISION 2026.8", fontsize=8, color=(0.4, 0.4, 0.4), fontname="helv")
        page.insert_text((500, 40), f"PAGE {p_idx+1} / 20", fontsize=8, color=(0.4, 0.4, 0.4), fontname="helv")
        
        # Chapter title
        chapter_num = (p_idx // 4) + 1
        page.insert_text((40, 80), f"Chapter {chapter_num}.{p_idx % 4 + 1}: Operations and Lifecycle Management", fontsize=15, fontname="helv")
        
        # Subheading
        page.insert_text((40, 105), f"Section {p_idx+1}: Diagnostic Procedures for Page Index {p_idx}", fontsize=11, color=(0.2, 0.4, 0.6), fontname="helv")
        
        # Body text
        p_body = (
            f"This is page {p_idx+1} of the 20-page large document qualification suite. "
            "High-page-count documents test memory stability, session caching, tab switching, and "
            "page thumbnail rendering in the Slint desktop UI. Each page contains independent vector shapes, "
            "metrics tables, and font runs. Rapid navigation across all 20 pages must maintain bounded RSS memory "
            "and sub-5ms GPU visible frame latencies."
        )
        page.insert_textbox(fitz.Rect(40, 125, 572, 210), p_body, fontsize=9.5, fontname="helv")
        
        # Graphic box
        shape = page.new_shape()
        shape.draw_rect(fitz.Rect(40, 220, 572, 380))
        shape.finish(color=(0.7, 0.7, 0.8), fill=(0.98, 0.98, 1.0), width=1)
        shape.commit()
        page.insert_text((55, 250), f"PAGE {p_idx+1} TELEMETRY & SYSTEM METRICS", fontsize=11, fontname="helv")
        page.insert_text((55, 280), f"Vector Elements: 14 | Glyph Count: 284 | Texture Allocations: 1", fontsize=9, fontname="courier")
        page.insert_text((55, 305), f"Target Scale: 100% (612 x 792 pt -> 612 x 792 px device coordinates)", fontsize=9, fontname="courier")
        page.insert_text((55, 330), f"Scissor Rect: [0, 0, 612, 792] | Hardware Clip: Active", fontsize=9, fontname="courier")
        page.insert_text((55, 355), f"Evidence Level: LOCAL_PARSED | Clean Execution Status: OK", fontsize=9, fontname="courier")
        
        # Bottom decorative vector bars
        shape = page.new_shape()
        for i in range(5):
            shape.draw_rect(fitz.Rect(40 + i*105, 410, 40 + (i+1)*105 - 10, 480))
            shape.finish(color=None, fill=(0.1 + i*0.15, 0.2 + i*0.1, 0.4 + i*0.1))
        shape.commit()
        
        page.insert_text((40, 760), "TOOLRAKYAT CONFIDENTIAL — AUTOMATED STRESS SUITE", fontsize=8, color=(0.6, 0.6, 0.6), fontname="helv")
        
    save_optimized(doc, CORPUS_DIR / "large-manual-20page.pdf")
    print("  -> Generated large-manual-20page.pdf (20 pages)")

# ----------------------------------------------------------------------
# 10. complex-infographic-graphics.pdf
# ----------------------------------------------------------------------
def gen_infographic():
    doc = fitz.open()
    page = doc.new_page(width=612, height=792)
    
    page.insert_text((40, 60), "COMPLEX GRAPHICS & VECTOR INFOGRAPHIC", fontsize=18, fontname="helv")
    page.insert_text((40, 85), "Bezier curves, alpha transparency, overlapping geometric shapes, and blend modes", fontsize=10, color=(0.4, 0.4, 0.4), fontname="helv")
    
    shape = page.new_shape()
    
    # 1. Concentric circles with alpha transparency
    center = fitz.Point(180, 240)
    for rad, col in [(90, (0.2, 0.5, 0.9)), (65, (0.9, 0.3, 0.3)), (40, (0.3, 0.8, 0.4))]:
        shape.draw_circle(center, rad)
        shape.finish(color=col, fill=col, width=2, fill_opacity=0.6)
        
    # 2. Complex Bezier Curves
    shape.draw_bezier(fitz.Point(320, 160), fitz.Point(400, 100), fitz.Point(460, 300), fitz.Point(540, 220))
    shape.finish(color=(0.8, 0.4, 0.0), width=4)
    
    shape.draw_bezier(fitz.Point(320, 240), fitz.Point(380, 340), fitz.Point(480, 120), fitz.Point(540, 280))
    shape.finish(color=(0.5, 0.1, 0.8), width=3)
    
    # 3. Polygon Star
    points = []
    star_center = fitz.Point(430, 420)
    for i in range(10):
        r = 60 if i % 2 == 0 else 25
        angle = i * math.pi / 5 - math.pi / 2
        points.append(fitz.Point(star_center.x + r * math.cos(angle), star_center.y + r * math.sin(angle)))
    points.append(points[0])
    shape.draw_polyline(points)
    shape.finish(color=(0.9, 0.7, 0.0), fill=(1.0, 0.9, 0.2), width=2, fill_opacity=0.8)
    
    # 4. Overlapping rounded rects
    for i in range(4):
        shape.draw_rect(fitz.Rect(60 + i*40, 360 + i*30, 200 + i*40, 460 + i*30))
        shape.finish(color=(0.2, 0.3, 0.6), fill=(0.1 + i*0.2, 0.3, 0.7 - i*0.1), width=1.5, fill_opacity=0.5)
        
    shape.commit()
    
    # Labels
    page.insert_text((90, 350), "Alpha Concentric Blends", fontsize=10, fontname="helv")
    page.insert_text((370, 340), "Cubic Bezier Splines", fontsize=10, fontname="helv")
    page.insert_text((380, 500), "10-Point Golden Star", fontsize=10, fontname="helv")
    page.insert_text((80, 520), "Translucent Stacking Hierarchy", fontsize=10, fontname="helv")
    
    save_optimized(doc, CORPUS_DIR / "complex-infographic-graphics.pdf")
    print("  -> Generated complex-infographic-graphics.pdf (1 page)")

# ----------------------------------------------------------------------
# Run all generators
# ----------------------------------------------------------------------
gen_invoice()
gen_report()
gen_scan()
gen_arabic_jawi()
gen_cjk()
gen_hebrew_bidi()
gen_form()
gen_annotations()
gen_large_manual()
gen_infographic()

print("All 10 Phase 8 Real-World Corpus fixtures generated successfully!")
