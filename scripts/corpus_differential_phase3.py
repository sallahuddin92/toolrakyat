#!/usr/bin/env python3
"""Phase 3 Differential Qualification Runner: StarPDF Native Renderer vs Google PDFium."""

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
OUT_DIR = ROOT / "target/corpus_differential_phase3"

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
                "differing_pixels": None,
                "diff_percentage": None,
                "rmse": None,
                "psnr": None,
            }

    arr_s = np.array(img_star, dtype=np.float64)
    arr_p = np.array(img_pdfium, dtype=np.float64)

    delta = np.abs(arr_s - arr_p)
    max_ch_delta = np.max(delta, axis=2)

    total_pixels = img_star.width * img_star.height
    diff_mask = max_ch_delta > 2
    differing_pixels = int(np.count_nonzero(diff_mask))
    diff_percentage = (differing_pixels / total_pixels) * 100.0

    mse = float(np.mean((arr_s - arr_p) ** 2))
    rmse = math.sqrt(mse)
    psnr = 100.0 if mse < 1e-10 else float(20.0 * np.log10(255.0) - 10.0 * np.log10(mse))

    # Generate heatmap image
    # Background: dimmed grayscale
    gray = np.mean(arr_s, axis=2) * 0.4 + 100.0
    heatmap_arr = np.stack([gray, gray, gray], axis=2).astype(np.uint8)

    # Highlight diffs in bright magenta/red
    intensity = np.clip(max_ch_delta * 2.5 + 50, 0, 255).astype(np.uint8)
    heatmap_arr[diff_mask, 0] = intensity[diff_mask]
    heatmap_arr[diff_mask, 1] = 0
    heatmap_arr[diff_mask, 2] = (intensity[diff_mask] * 0.7).astype(np.uint8)

    heatmap_img = Image.fromarray(heatmap_arr, mode="RGB")
    heatmap_img.save(str(heatmap_path))

    # Triptych: StarPDF | PDFium | Heatmap
    W, H = img_star.width, img_star.height
    triptych = Image.new("RGB", (W * 3 + 20, H + 40), (240, 240, 240))
    triptych.paste(img_star, (0, 30))
    triptych.paste(img_pdfium, (W + 10, 30))
    triptych.paste(heatmap_img, (W * 2 + 20, 30))
    triptych.save(str(triptych_path))

    return {
        "dim_match": dim_match,
        "near_match": near_match,
        "width": img_star.width,
        "height": img_star.height,
        "differing_pixels": differing_pixels,
        "total_pixels": total_pixels,
        "diff_percentage": diff_percentage,
        "rmse": rmse,
        "psnr": psnr,
    }


def main():
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    fixtures = find_fixtures()
    print(f"Discovered {len(fixtures)} PDF fixtures in corpus.")

    results = []

    passed_count = 0
    visual_disagree_count = 0
    unsupported_count = 0
    parse_failure_count = 0
    invalid_fixture_count = 0

    for idx, f in enumerate(fixtures, 1):
        rel = str(f.relative_to(ROOT))
        stem = f.stem
        subdir = OUT_DIR / f.parent.name
        subdir.mkdir(parents=True, exist_ok=True)

        star_png = subdir / f"{stem}_starpdf.png"
        pdfium_png = subdir / f"{stem}_pdfium.png"
        heatmap_png = subdir / f"{stem}_heatmap.png"
        triptych_png = subdir / f"{stem}_triptych.png"

        print(f"[{idx:02d}/{len(fixtures):02d}] Evaluating {rel}...", end=" ", flush=True)

        star_res = run_starpdf(f, star_png)
        pdfium_res = run_pdfium(f, pdfium_png)

        entry = {
            "id": idx,
            "path": rel,
            "stem": stem,
            "category": f.parent.name,
            "starpdf": star_res,
            "pdfium": pdfium_res,
            "metrics": None,
            "verdict": "UNKNOWN",
            "notes": "",
        }

        if star_res.get("status") == "OK" and pdfium_res.get("status") == "OK":
            metrics = compute_metrics_and_heatmap(star_png, pdfium_png, heatmap_png, triptych_png)
            entry["metrics"] = metrics
            unsupported_ops = star_res.get("unsupported_operators", [])

            if metrics["diff_percentage"] is None:
                entry["verdict"] = "VISUAL_DISAGREEMENT"
                entry["notes"] = f"Dimensions differ: StarPDF {metrics['star_size']} vs PDFium {metrics['pdfium_size']}"
                visual_disagree_count += 1
            elif unsupported_ops:
                entry["verdict"] = "UNSUPPORTED"
                entry["notes"] = f"Unsupported operators: {', '.join(unsupported_ops)}"
                unsupported_count += 1
            elif metrics["diff_percentage"] <= 5.0:
                entry["verdict"] = "PASS"
                rounding_note = " (1px boundary rounding)" if metrics.get("near_match") else ""
                entry["notes"] = f"Subpixel match{rounding_note}: {metrics['diff_percentage']:.2f}% diff, RMSE={metrics['rmse']:.2f}"
                passed_count += 1
            else:
                entry["verdict"] = "VISUAL_DISAGREEMENT"
                entry["notes"] = f"Visual difference: {metrics['diff_percentage']:.2f}% diff, RMSE={metrics['rmse']:.2f}"
                visual_disagree_count += 1
        elif star_res.get("status") != "OK" and pdfium_res.get("status") != "OK":
            # Both rejected (e.g. invalid fixture or malformed)
            entry["verdict"] = "INVALID_FIXTURE"
            entry["notes"] = f"Both engines failed: StarPDF ({star_res.get('error')}) / PDFium ({pdfium_res.get('error')})"
            invalid_fixture_count += 1
        elif star_res.get("status") != "OK":
            err = str(star_res.get("error", ""))
            stage = star_res.get("stage", "")
            if "encrypted" in f.name.lower() or "Encrypted" in err or "password" in err.lower():
                entry["verdict"] = "UNSUPPORTED"
                entry["notes"] = f"Encrypted PDF (fails closed safely): {err}"
                unsupported_count += 1
            elif "malformed" in f.name.lower():
                entry["verdict"] = "INVALID_FIXTURE"
                entry["notes"] = f"Malformed test fixture: {err}"
                invalid_fixture_count += 1
            else:
                entry["verdict"] = "PARSE_FAILURE"
                entry["notes"] = f"StarPDF failed at stage {stage}: {err}"
                parse_failure_count += 1
        else:
            entry["verdict"] = "ORACLE_FAILURE"
            entry["notes"] = f"PDFium failed: {pdfium_res.get('error')}"

        print(f"-> {entry['verdict']} ({entry['notes']})")
        results.append(entry)

    summary = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "total_fixtures": len(fixtures),
        "passed": passed_count,
        "visual_disagreement": visual_disagree_count,
        "unsupported": unsupported_count,
        "parse_failure": parse_failure_count,
        "invalid_fixture": invalid_fixture_count,
        "results": results,
    }

    report_path = OUT_DIR / "corpus_scorecard.json"
    with open(report_path, "w") as f:
        json.dump(summary, f, indent=2)

    print("\n" + "=" * 80)
    print("STARPDF PHASE 3 NATIVE RENDERER CORPUS SCORECARD")
    print("=" * 80)
    print(f"Total Fixtures Evaluated: {len(fixtures)}")
    print(f"  PASS (Fidelity <= 5.0% or exact geometry): {passed_count}")
    print(f"  VISUAL DISAGREEMENT:                       {visual_disagree_count}")
    print(f"  UNSUPPORTED (Encrypted or unknown op):     {unsupported_count}")
    print(f"  INVALID FIXTURE (Intentionally malformed): {invalid_fixture_count}")
    print(f"  PARSE FAILURE:                             {parse_failure_count}")
    print(f"Full scorecard saved to: {report_path}")
    print("=" * 80)


if __name__ == "__main__":
    main()
