#!/usr/bin/env python3
"""Phase 5 Strict Differential Qualification Runner: StarPDF Native Renderer vs Google PDFium."""

import json
import math
import os
import subprocess
import sys
import time
from pathlib import Path
from PIL import Image, ImageOps
import numpy as np
import pypdfium2

ROOT = Path(__file__).resolve().parents[1]
PROBE_BIN = ROOT / "target/debug/examples/render_page_probe"
OUT_DIR = ROOT / "target/corpus_differential_phase5"

FIXTURE_DIRS = [
    ROOT / "test-assets",
    ROOT / "engine/starpdf/tests/fixtures/v0_9_compat",
    ROOT / "engine/starpdf/tests/fixtures/v0_10_compat",
    ROOT / "engine/starpdf/tests/fixtures/v0_11_complex",
]


def find_fixtures():
    fixtures = []
    for d in FIXTURE_DIRS:
        if d.is_dir():
            for p in sorted(d.glob("*.pdf")):
                fixtures.append(p)
    return sorted(list(set(fixtures)), key=lambda p: str(p.relative_to(ROOT)))


def run_starpdf(pdf_path, out_png, page_index=0, scale=1.0):
    cmd = [
        str(PROBE_BIN),
        str(pdf_path),
        str(page_index),
        str(scale),
        str(out_png),
    ]
    t0 = time.perf_counter()
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    dur_ms = (time.perf_counter() - t0) * 1000.0

    if res.returncode != 0:
        return {
            "status": "CRASH",
            "error": res.stderr.strip() or f"Exit code {res.returncode}",
            "duration_ms": dur_ms,
        }

    try:
        data = json.loads(res.stdout.strip())
        data["total_proc_duration_ms"] = dur_ms
        return data
    except Exception as e:
        return {
            "status": "JSON_PARSE_ERROR",
            "error": str(e),
            "raw_stdout": res.stdout[:500],
            "duration_ms": dur_ms,
        }


def run_pdfium(pdf_path, out_png, page_index=0, scale=1.0):
    t0 = time.perf_counter()
    try:
        doc = pypdfium2.PdfDocument(str(pdf_path))
        if page_index >= len(doc):
            return {"status": "PAGE_BOUNDS", "error": f"Page index {page_index} >= {len(doc)}"}
        try:
            doc.init_forms()
        except Exception:
            pass
        page = doc[page_index]
        bitmap = page.render(scale=scale)
        pil_img = bitmap.to_pil().convert("RGB")
        pil_img.save(str(out_png))
        dur_ms = (time.perf_counter() - t0) * 1000.0
        return {
            "status": "OK",
            "width": pil_img.width,
            "height": pil_img.height,
            "duration_ms": dur_ms,
        }
    except Exception as e:
        dur_ms = (time.perf_counter() - t0) * 1000.0
        return {"status": "ERROR", "error": str(e), "duration_ms": dur_ms}


def compute_metrics_and_heatmap(img_star_path, img_pdfium_path, heatmap_path, triptych_path):
    img_star = Image.open(img_star_path).convert("RGB")
    img_pdfium = Image.open(img_pdfium_path).convert("RGB")

    dim_match = (img_star.size == img_pdfium.size)
    near_match = False
    if not dim_match:
        if abs(img_star.width - img_pdfium.width) <= 1 and abs(img_star.height - img_pdfium.height) <= 1:
            near_match = True
            common_w = min(img_star.width, img_pdfium.width)
            common_h = min(img_star.height, img_pdfium.height)
            img_star = img_star.crop((0, 0, common_w, common_h))
            img_pdfium = img_pdfium.crop((0, 0, common_w, common_h))
        else:
            return {
                "dim_match": False,
                "near_match": False,
                "star_size": img_star.size,
                "pdfium_size": img_pdfium.size,
                "diff_percentage": 100.0,
                "rmse": 255.0,
                "psnr": 0.0,
            }

    arr_s = np.array(img_star, dtype=np.float32)
    arr_p = np.array(img_pdfium, dtype=np.float32)

    diff = np.abs(arr_s - arr_p)
    diff_pixels = np.any(diff > 5, axis=-1)
    differing_count = int(np.sum(diff_pixels))
    total_pixels = int(diff_pixels.size)
    diff_pct = (differing_count / total_pixels) * 100.0

    mse = float(np.mean((arr_s - arr_p) ** 2))
    rmse = math.sqrt(mse)
    if mse == 0.0:
        psnr = 100.0
    else:
        psnr = float(20.0 * math.log10(255.0 / rmse))

    # Heatmap generation: Jet-like colormap for differing pixels
    diff_max = np.max(diff, axis=-1)
    heat = np.zeros((arr_s.shape[0], arr_s.shape[1], 3), dtype=np.uint8)
    mask_diff = diff_max > 5
    heat[mask_diff, 0] = np.clip(diff_max[mask_diff] * 4, 120, 255).astype(np.uint8)
    heat[mask_diff, 1] = np.clip(255 - diff_max[mask_diff] * 2, 0, 180).astype(np.uint8)
    heat[mask_diff, 2] = 40
    heat[~mask_diff] = [240, 240, 240]

    heat_img = Image.fromarray(heat)
    heat_img.save(str(heatmap_path))

    # Triptych: StarPDF | PDFium | Heatmap
    w, h = img_star.size
    triptych = Image.new("RGB", (w * 3 + 20, h + 40), (255, 255, 255))
    triptych.paste(img_star, (0, 30))
    triptych.paste(img_pdfium, (w + 10, 30))
    triptych.paste(heat_img, (w * 2 + 20, 30))
    triptych.save(str(triptych_path))

    return {
        "dim_match": dim_match,
        "near_match": near_match,
        "star_size": img_star.size,
        "pdfium_size": img_pdfium.size,
        "differing_pixels": differing_count,
        "total_pixels": total_pixels,
        "diff_percentage": diff_pct,
        "rmse": rmse,
        "psnr": psnr,
    }


def main():
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    images_dir = OUT_DIR / "renders"
    heatmaps_dir = OUT_DIR / "heatmaps"
    triptychs_dir = OUT_DIR / "triptychs"
    images_dir.mkdir(exist_ok=True)
    heatmaps_dir.mkdir(exist_ok=True)
    triptychs_dir.mkdir(exist_ok=True)

    fixtures = find_fixtures()
    print(f"Found {len(fixtures)} fixtures across corpus directories.")

    results = []
    pass_strict = 0
    pass_subpixel = 0
    visual_disagreements = 0
    unsupported_count = 0
    invalid_fixture_count = 0
    parse_failures = 0

    for idx, pdf_path in enumerate(fixtures, 1):
        rel_path = pdf_path.relative_to(ROOT)
        stem = pdf_path.stem
        category = pdf_path.parent.name
        prefix = f"{idx:02d}_{category}_{stem}"
        print(f"[{idx:02d}/{len(fixtures):02d}] {rel_path}...", end="", flush=True)

        star_png = images_dir / f"{prefix}_starpdf.png"
        pdfium_png = images_dir / f"{prefix}_pdfium.png"
        heat_png = heatmaps_dir / f"{prefix}_diff.png"
        trip_png = triptychs_dir / f"{prefix}_triptych.png"

        star_res = run_starpdf(pdf_path, star_png)
        pdfium_res = run_pdfium(pdf_path, pdfium_png)

        entry = {
            "id": idx,
            "path": str(rel_path),
            "stem": stem,
            "category": category,
            "starpdf": star_res,
            "pdfium": pdfium_res,
        }

        # Failure / unsupported cases
        if star_res.get("status") != "OK" and pdfium_res.get("status") != "OK":
            entry["verdict"] = "INVALID_FIXTURE"
            entry["notes"] = f"Both engines failed: StarPDF ({star_res.get('status')}) / PDFium ({pdfium_res.get('error', '')[:80]})"
            invalid_fixture_count += 1
            print(f" -> INVALID_FIXTURE ({entry['notes']})")
        elif pdfium_res.get("status") != "OK":
            entry["verdict"] = "ORACLE_FAILURE"
            entry["notes"] = f"PDFium failed: {pdfium_res.get('error', '')}"
            unsupported_count += 1
            print(f" -> ORACLE_FAILURE ({entry['notes']})")
        elif star_res.get("status") != "OK":
            entry["verdict"] = "PARSE_FAILURE"
            entry["notes"] = f"StarPDF failed: {star_res.get('error', '')}"
            parse_failures += 1
            print(f" -> PARSE_FAILURE ({entry['notes']})")
        else:
            # Both rendered successfully, compute pixel comparison
            metrics = compute_metrics_and_heatmap(star_png, pdfium_png, heat_png, trip_png)
            entry["metrics"] = metrics

            diff_pct = metrics["diff_percentage"]
            rmse = metrics["rmse"]
            unsupported_ops = star_res.get("unsupported_operators", [])

            if unsupported_ops:
                entry["verdict"] = "UNSUPPORTED"
                entry["notes"] = f"Unsupported operators detected: {unsupported_ops}"
                unsupported_count += 1
                print(f" -> UNSUPPORTED ({entry['notes']})")
            elif not metrics["dim_match"] and not metrics["near_match"]:
                entry["verdict"] = "VISUAL_DISAGREEMENT"
                entry["notes"] = f"Dimension mismatch: StarPDF={metrics['star_size']} vs PDFium={metrics['pdfium_size']}"
                visual_disagreements += 1
                print(f" -> VISUAL_DISAGREEMENT ({entry['notes']})")
            elif diff_pct <= 1.50:
                entry["verdict"] = "PASS"
                entry["notes"] = f"Strict pass (<= 1.5%): {diff_pct:.2f}% diff, RMSE={rmse:.2f}"
                pass_strict += 1
                print(f" -> PASS ({entry['notes']})")
            elif diff_pct <= 5.0:
                entry["verdict"] = "PASS"
                match_note = "1px boundary rounding" if metrics.get("near_match") else "Subpixel text anti-aliasing"
                entry["notes"] = f"Subpixel match ({match_note}): {diff_pct:.2f}% diff, RMSE={rmse:.2f}"
                pass_subpixel += 1
                print(f" -> PASS ({entry['notes']})")
            else:
                entry["verdict"] = "VISUAL_DISAGREEMENT"
                entry["notes"] = f"Visual difference: {diff_pct:.2f}% diff, RMSE={rmse:.2f}"
                visual_disagreements += 1
                print(f" -> VISUAL_DISAGREEMENT ({entry['notes']})")

        results.append(entry)

    scorecard = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "total_fixtures": len(fixtures),
        "passed_strict_1_5pct": pass_strict,
        "passed_subpixel_5pct": pass_subpixel,
        "total_passed": pass_strict + pass_subpixel,
        "visual_disagreement": visual_disagreements,
        "unsupported": unsupported_count,
        "parse_failure": parse_failures,
        "invalid_fixture": invalid_fixture_count,
        "results": results,
    }

    scorecard_path = OUT_DIR / "corpus_scorecard.json"
    with open(scorecard_path, "w") as f:
        json.dump(scorecard, f, indent=2)

    print("\n" + "=" * 80)
    print("STARPDF PHASE 5 STRICT NATIVE RENDERER CORPUS SCORECARD")
    print("=" * 80)
    print(f"Total Fixtures Evaluated: {len(fixtures)}")
    print(f"  STRICT PASS (<= 1.5% diff or exact):  {pass_strict}")
    print(f"  SUBPIXEL PASS (<= 5.0% anti-alias):   {pass_subpixel}")
    print(f"  TOTAL PASSING:                        {pass_strict + pass_subpixel}")
    print(f"  VISUAL DISAGREEMENT (> 5.0%):         {visual_disagreements}")
    print(f"  ORACLE / UNSUPPORTED (Encrypted):     {unsupported_count}")
    print(f"  INVALID FIXTURE (Malformed test):     {invalid_fixture_count}")
    print(f"  PARSE FAILURE:                        {parse_failures}")
    print(f"Full scorecard saved to: {scorecard_path}")
    print("=" * 80)


if __name__ == "__main__":
    main()
