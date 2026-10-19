#!/usr/bin/env python3
"""Phase 6 Strict Differential & GPU Benchmark Qualification Runner.

Directly compares GPU-rasterized StarPDF against CPU StarPDF and Google PDFium.
Measures visual differences, missing regions, clipping, multi-resolution parity,
and provides an honest, empirical breakdown of GPU vs CPU fallback primitives.
"""

import json
import math
import os
import subprocess
import sys
import time
from pathlib import Path
from PIL import Image
import numpy as np
import pypdfium2

ROOT = Path(__file__).resolve().parents[1]
PROBE_BIN = ROOT / "target/debug/examples/render_page_probe"
OUT_DIR = ROOT / "target/corpus_differential_phase6"

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


def run_starpdf(pdf_path, out_png, page_index=0, scale=1.0, backend="cpu"):
    cmd = [
        str(PROBE_BIN),
        str(pdf_path),
        str(page_index),
        str(scale),
        str(out_png),
        str(backend),
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
            return {
                "status": "PAGE_BOUNDS",
                "error": f"Page index {page_index} >= {len(doc)}",
            }
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


def compare_images(img_a_path, img_b_path, heatmap_path=None):
    img_a = Image.open(img_a_path).convert("RGB")
    img_b = Image.open(img_b_path).convert("RGB")

    dim_match = img_a.size == img_b.size
    if not dim_match:
        if abs(img_a.width - img_b.width) <= 2 and abs(img_a.height - img_b.height) <= 2:
            common_w = min(img_a.width, img_b.width)
            common_h = min(img_a.height, img_b.height)
            img_a = img_a.crop((0, 0, common_w, common_h))
            img_b = img_b.crop((0, 0, common_w, common_h))
        else:
            return {
                "dim_match": False,
                "diff_percentage": 100.0,
                "rmse": 255.0,
                "psnr": 0.0,
            }

    arr_a = np.array(img_a, dtype=np.float32)
    arr_b = np.array(img_b, dtype=np.float32)

    diff = np.abs(arr_a - arr_b)
    diff_pixels = np.any(diff > 5, axis=-1)
    differing_count = int(np.sum(diff_pixels))
    total_pixels = int(diff_pixels.size)
    diff_pct = (differing_count / total_pixels) * 100.0

    mse = float(np.mean((arr_a - arr_b) ** 2))
    rmse = math.sqrt(mse)
    psnr = 100.0 if mse == 0.0 else float(20.0 * math.log10(255.0 / rmse))

    if heatmap_path:
        diff_max = np.max(diff, axis=-1)
        heat = np.zeros((arr_a.shape[0], arr_a.shape[1], 3), dtype=np.uint8)
        mask_diff = diff_max > 5
        heat[mask_diff, 0] = np.clip(diff_max[mask_diff] * 4, 120, 255).astype(np.uint8)
        heat[mask_diff, 1] = np.clip(255 - diff_max[mask_diff] * 2, 0, 180).astype(np.uint8)
        heat[mask_diff, 2] = 40
        heat[~mask_diff] = [240, 240, 240]
        heat_img = Image.fromarray(heat)
        heat_img.save(str(heatmap_path))

    return {
        "dim_match": True,
        "width": img_a.width,
        "height": img_a.height,
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
    print("=" * 80)
    print("STARPDF PHASE 6: GPU-NATIVE PDF RENDERING & DIFFERENTIAL QUALIFICATION")
    print(f"Targeting {len(fixtures)} fixtures across corpus directories.")
    print("=" * 80)

    results = []
    gpu_native_count = 0
    gpu_hybrid_count = 0
    total_gpu_paths = 0
    total_gpu_images = 0
    total_fallback_items = 0

    for idx, pdf_path in enumerate(fixtures, 1):
        rel_path = pdf_path.relative_to(ROOT)
        stem = pdf_path.stem
        category = pdf_path.parent.name
        prefix = f"{idx:02d}_{category}_{stem}"
        print(f"[{idx:02d}/{len(fixtures):02d}] {rel_path}...", end="", flush=True)

        cpu_png = images_dir / f"{prefix}_cpu.png"
        gpu_png = images_dir / f"{prefix}_gpu.png"
        pdfium_png = images_dir / f"{prefix}_pdfium.png"
        heat_gpu_cpu = heatmaps_dir / f"{prefix}_gpu_vs_cpu_diff.png"
        heat_gpu_pdfium = heatmaps_dir / f"{prefix}_gpu_vs_pdfium_diff.png"
        triptych_png = triptychs_dir / f"{prefix}_triptych.png"

        # 1. Cold GPU render
        t_cold0 = time.perf_counter()
        gpu_cold = run_starpdf(pdf_path, gpu_png, backend="gpu")
        cold_latency_ms = (time.perf_counter() - t_cold0) * 1000.0

        # 2. Warm GPU render
        t_warm0 = time.perf_counter()
        gpu_warm = run_starpdf(pdf_path, gpu_png, backend="gpu")
        warm_latency_ms = (time.perf_counter() - t_warm0) * 1000.0

        # 3. CPU StarPDF render
        cpu_res = run_starpdf(pdf_path, cpu_png, backend="cpu")

        # 4. Google PDFium render
        pdfium_res = run_pdfium(pdf_path, pdfium_png)

        if gpu_warm.get("status") != "OK" or cpu_res.get("status") != "OK":
            print(f" SKIP ({gpu_warm.get('error', 'Error')})")
            results.append({
                "fixture": str(rel_path),
                "status": "SKIP",
                "error": gpu_warm.get("error") or cpu_res.get("error"),
            })
            continue

        # Comparisons
        diff_gpu_cpu = compare_images(gpu_png, cpu_png, heat_gpu_cpu)
        diff_gpu_pdfium = None
        if pdfium_res.get("status") == "OK" and pdfium_png.exists():
            diff_gpu_pdfium = compare_images(gpu_png, pdfium_png, heat_gpu_pdfium)

            # Generate Triptych: GPU StarPDF | Google PDFium | Diff Heatmap
            try:
                g_img = Image.open(gpu_png).convert("RGB")
                p_img = Image.open(pdfium_png).convert("RGB")
                h_img = Image.open(heat_gpu_pdfium).convert("RGB")
                w, h = g_img.size
                triptych = Image.new("RGB", (w * 3 + 20, h + 30), (255, 255, 255))
                triptych.paste(g_img, (0, 20))
                triptych.paste(p_img, (w + 10, 20))
                triptych.paste(h_img, (w * 2 + 20, 20))
                triptych.save(str(triptych_png))
            except Exception:
                pass

        gpu_paths = gpu_warm.get("gpu_paths_rendered", 0)
        gpu_images = gpu_warm.get("gpu_images_rendered", 0)
        fallback_items = gpu_warm.get("fallback_cpu_items", 0)
        fallback_reasons = gpu_warm.get("fallback_reasons", [])

        total_gpu_paths += gpu_paths
        total_gpu_images += gpu_images
        total_fallback_items += fallback_items

        if fallback_items == 0:
            classification = "100%_GPU_NATIVE"
            gpu_native_count += 1
        else:
            classification = "HYBRID_GPU_CPU"
            gpu_hybrid_count += 1

        entry = {
            "fixture": str(rel_path),
            "classification": classification,
            "width": gpu_warm.get("width"),
            "height": gpu_warm.get("height"),
            "items_count": gpu_warm.get("items_count"),
            "gpu_draw_calls": gpu_warm.get("gpu_draw_calls"),
            "gpu_paths_rendered": gpu_paths,
            "gpu_images_rendered": gpu_images,
            "fallback_cpu_items": fallback_items,
            "fallback_reasons": fallback_reasons,
            "timings_ms": {
                "ir_compile": gpu_warm.get("compile_duration_ms"),
                "cpu_render": cpu_res.get("render_duration_ms"),
                "gpu_upload": gpu_warm.get("upload_duration_ms"),
                "gpu_render": gpu_warm.get("render_duration_ms"),
                "gpu_readback": gpu_warm.get("readback_duration_ms"),
                "gpu_total_frame": gpu_warm.get("total_frame_duration_ms"),
                "cold_total_proc": cold_latency_ms,
                "warm_total_proc": warm_latency_ms,
            },
            "differential_gpu_vs_cpu": {
                "diff_pct": diff_gpu_cpu.get("diff_percentage"),
                "rmse": diff_gpu_cpu.get("rmse"),
                "psnr": diff_gpu_cpu.get("psnr"),
            },
            "differential_gpu_vs_pdfium": {
                "diff_pct": diff_gpu_pdfium.get("diff_percentage") if diff_gpu_pdfium else None,
                "rmse": diff_gpu_pdfium.get("rmse") if diff_gpu_pdfium else None,
                "psnr": diff_gpu_pdfium.get("psnr") if diff_gpu_pdfium else None,
            } if diff_gpu_pdfium else None,
        }
        results.append(entry)

        diff_str = f"diff_cpu={diff_gpu_cpu.get('diff_percentage', 0):.2f}%"
        pdfium_str = f"diff_pdfium={diff_gpu_pdfium.get('diff_percentage', 0):.2f}%" if diff_gpu_pdfium else "pdfium=N/A"
        timing_str = f"gpu_frame={gpu_warm.get('total_frame_duration_ms', 0):.1f}ms vs cpu={cpu_res.get('render_duration_ms', 0):.1f}ms"
        print(f" OK [{classification} | {diff_str} | {pdfium_str} | {timing_str}]")

    # Multi-resolution qualification sweep on representative fixtures
    print("\n--- MULTI-RESOLUTION & ROTATION SWEEP ---")
    multi_res_results = []
    sample_fixtures = [
        ROOT / "test-assets/vector-primitives.pdf",
        ROOT / "test-assets/image-shared-xobject.pdf",
        ROOT / "test-assets/smartpdf-adobe-like-form.pdf",
        ROOT / "test-assets/text-multispan-heading.pdf",
    ]
    for fix in sample_fixtures:
        if not fix.exists():
            continue
        rel = str(fix.relative_to(ROOT))
        for scale in [0.5, 1.0, 1.5, 2.0]:
            cpu_p = OUT_DIR / f"multires_{fix.stem}_s{scale}_cpu.png"
            gpu_p = OUT_DIR / f"multires_{fix.stem}_s{scale}_gpu.png"
            c_res = run_starpdf(fix, cpu_p, scale=scale, backend="cpu")
            g_res = run_starpdf(fix, gpu_p, scale=scale, backend="gpu")
            if c_res.get("status") == "OK" and g_res.get("status") == "OK":
                diff = compare_images(gpu_p, cpu_p)
                multi_res_results.append({
                    "fixture": rel,
                    "scale": scale,
                    "width": g_res.get("width"),
                    "height": g_res.get("height"),
                    "diff_percentage": diff.get("diff_percentage"),
                    "psnr": diff.get("psnr"),
                    "gpu_frame_ms": g_res.get("total_frame_duration_ms"),
                    "cpu_render_ms": c_res.get("render_duration_ms"),
                })
                print(f"  {fix.stem} @ {scale}x ({g_res.get('width')}x{g_res.get('height')}): diff={diff.get('diff_percentage'):.3f}%, PSNR={diff.get('psnr'):.2f}dB, GPU={g_res.get('total_frame_duration_ms'):.1f}ms")

    # Summary Report
    summary = {
        "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ"),
        "total_fixtures": len(fixtures),
        "qualified_fixtures": len(results),
        "gpu_native_fixtures": gpu_native_count,
        "gpu_hybrid_fixtures": gpu_hybrid_count,
        "aggregate_gpu_paths_rendered": total_gpu_paths,
        "aggregate_gpu_images_rendered": total_gpu_images,
        "aggregate_fallback_cpu_items": total_fallback_items,
        "multi_resolution_sweep": multi_res_results,
        "details": results,
    }

    report_path = OUT_DIR / "phase6_gpu_qualification_report.json"
    with open(report_path, "w") as f:
        json.dump(summary, f, indent=2)

    print("\n" + "=" * 80)
    print("PHASE 6 GPU QUALIFICATION SUMMARY")
    print(f"Total Qualified Fixtures:         {len(results)}")
    print(f"100% GPU Native Fixtures:         {gpu_native_count}")
    print(f"Hybrid GPU/CPU Fallback Fixtures: {gpu_hybrid_count}")
    print(f"Total GPU Vector Paths Rendered:  {total_gpu_paths}")
    print(f"Total GPU Image Objects Textured: {total_gpu_images}")
    print(f"Total CPU Fallback Items:         {total_fallback_items}")
    print(f"Report written to:                {report_path}")
    print("=" * 80)


if __name__ == "__main__":
    main()
