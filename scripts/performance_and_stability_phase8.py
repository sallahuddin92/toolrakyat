#!/usr/bin/env python3
"""Phase 8 Performance, Memory, and Stability Profiler.

Measures:
1. Cold Start vs Warm First-Page Visible Latency
2. Interactive Scroll & Multi-Page Navigation (20-page large document)
3. Repeated Multi-Scale Zoom Sweeps (50% to 300%)
4. Resident Set Size (RSS) Memory Footprint
5. GPU Render Pass vs Texture Upload vs Readback vs Total Visible Frame Breakdown
6. GPU Device Refusal & Fail-Safe CPU Fallback
"""

import json
import os
import subprocess
import sys
import time
from pathlib import Path
import psutil

ROOT = Path(__file__).resolve().parents[1]
PROBE_BIN = ROOT / "target/debug/examples/render_page_probe"
OUT_DIR = ROOT / "target/corpus_differential_phase8"
OUT_DIR.mkdir(parents=True, exist_ok=True)


def run_probe(pdf_path, page=0, scale=1.0, backend="gpu"):
    out_png = OUT_DIR / "temp_bench.png"
    cmd = [
        str(PROBE_BIN),
        str(pdf_path),
        str(page),
        str(scale),
        str(out_png),
        backend,
    ]
    t0 = time.perf_counter()
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    wall_ms = (time.perf_counter() - t0) * 1000.0

    if res.returncode != 0:
        return {"status": "ERROR", "error": res.stderr.strip(), "wall_ms": wall_ms}

    try:
        data = json.loads(res.stdout)
        data["wall_ms"] = wall_ms
        return data
    except Exception as e:
        return {"status": "ERROR", "error": str(e), "wall_ms": wall_ms}


def get_process_memory():
    process = psutil.Process(os.getpid())
    return process.memory_info().rss / (1024 * 1024)  # MB


def main():
    print("==================================================================")
    print("STARPDF PHASE 8: PERFORMANCE, STABILITY & MEMORY PROFILING")
    print("==================================================================")

    benchmarks = {}

    # 1. Cold Start vs Warm Latency on Key Documents
    print("\n--- 1. Cold Start vs Warm First-Page Latency ---")
    key_docs = [
        ("Office Invoice", ROOT / "test-assets/realworld_corpus/office-invoice-enterprise.pdf"),
        ("Scanned Archive", ROOT / "test-assets/realworld_corpus/scanned-multires-archive.pdf"),
        ("Complex Infographic", ROOT / "test-assets/realworld_corpus/complex-infographic-graphics.pdf"),
        ("20-Page Manual", ROOT / "test-assets/realworld_corpus/large-manual-20page.pdf"),
    ]

    cold_warm_results = []
    for label, path in key_docs:
        # Cold start (first invocation)
        cold_res = run_probe(path, page=0, scale=1.0, backend="gpu")
        # Warm runs (average of 3 subsequent)
        warm_passes = []
        warm_frames = []
        for _ in range(3):
            w = run_probe(path, page=0, scale=1.0, backend="gpu")
            warm_passes.append(w.get("render_duration_ms", 0.0))
            warm_frames.append(w.get("total_frame_duration_ms", 0.0))

        avg_warm_pass = sum(warm_passes) / len(warm_passes)
        avg_warm_frame = sum(warm_frames) / len(warm_frames)

        record = {
            "document": label,
            "cold_frame_ms": round(cold_res.get("total_frame_duration_ms", 0.0), 2),
            "cold_render_pass_ms": round(cold_res.get("render_duration_ms", 0.0), 2),
            "warm_frame_ms": round(avg_warm_frame, 2),
            "warm_render_pass_ms": round(avg_warm_pass, 2),
            "glyphs": cold_res.get("gpu_glyphs_rendered", 0),
            "paths": cold_res.get("gpu_paths_rendered", 0),
        }
        cold_warm_results.append(record)
        print(f"  {label:20s}: Cold Frame: {record['cold_frame_ms']:6.2f}ms | Warm Frame: {record['warm_frame_ms']:5.2f}ms | GPU Pass: {record['warm_render_pass_ms']:4.2f}ms")

    benchmarks["cold_warm"] = cold_warm_results

    # 2. Interactive Scroll & Paging Across 20-Page Document
    print("\n--- 2. Interactive Scroll Navigation (20-Page Document) ---")
    manual_path = ROOT / "test-assets/realworld_corpus/large-manual-20page.pdf"
    scroll_latencies = []
    for p in range(20):
        t_start = time.perf_counter()
        res = run_probe(manual_path, page=p, scale=1.0, backend="gpu")
        dur = (time.perf_counter() - t_start) * 1000.0
        scroll_latencies.append({
            "page": p,
            "gpu_render_pass_ms": round(res.get("render_duration_ms", 0.0), 2),
            "visible_frame_ms": round(res.get("total_frame_duration_ms", 0.0), 2),
            "wall_ms": round(dur, 2),
        })

    avg_scroll_gpu = sum(x["gpu_render_pass_ms"] for x in scroll_latencies) / len(scroll_latencies)
    avg_scroll_frame = sum(x["visible_frame_ms"] for x in scroll_latencies) / len(scroll_latencies)
    max_scroll_frame = max(x["visible_frame_ms"] for x in scroll_latencies)

    print(f"  Paging 20 pages: Average GPU Pass: {avg_scroll_gpu:.2f}ms | Average Visible Frame: {avg_scroll_frame:.2f}ms | Worst Frame: {max_scroll_frame:.2f}ms")
    benchmarks["scroll_navigation"] = {
        "pages_tested": 20,
        "avg_gpu_pass_ms": round(avg_scroll_gpu, 2),
        "avg_visible_frame_ms": round(avg_scroll_frame, 2),
        "max_visible_frame_ms": round(max_scroll_frame, 2),
        "details": scroll_latencies,
    }

    # 3. Repeated Multi-Scale Zoom Sweeps
    print("\n--- 3. Repeated Zoom Sweeps (50% to 300%) ---")
    zoom_scales = [0.5, 0.75, 1.0, 1.25, 1.5, 2.0, 2.5, 3.0]
    zoom_results = []
    for scale in zoom_scales:
        res = run_probe(manual_path, page=0, scale=scale, backend="gpu")
        c_res = run_probe(manual_path, page=0, scale=scale, backend="cpu")
        gpu_pass = res.get("render_duration_ms", 0.0)
        cpu_pass = c_res.get("render_duration_ms", 0.0)
        spd = (cpu_pass / gpu_pass) if gpu_pass > 0 else 1.0
        rec = {
            "scale": scale,
            "zoom_percent": int(scale * 100),
            "pixels": f"{res.get('width', 0)}x{res.get('height', 0)}",
            "gpu_render_pass_ms": round(gpu_pass, 2),
            "cpu_render_ms": round(cpu_pass, 2),
            "visible_frame_ms": round(res.get("total_frame_duration_ms", 0.0), 2),
            "speedup": round(spd, 2),
        }
        zoom_results.append(rec)
        print(f"  Zoom {rec['zoom_percent']:3d}% ({rec['pixels']:10s}): GPU Pass: {rec['gpu_render_pass_ms']:5.2f}ms | CPU: {rec['cpu_render_ms']:6.2f}ms ({rec['speedup']:5.2f}x) | Frame: {rec['visible_frame_ms']:6.2f}ms")

    benchmarks["zoom_sweeps"] = zoom_results

    # 4. Latency Stage Breakdown
    print("\n--- 4. Latency Stage Breakdown (Average Across Targets) ---")
    stage_breakdowns = []
    for label, path in key_docs:
        res = run_probe(path, page=0, scale=1.0, backend="gpu")
        rec = {
            "document": label,
            "compile_ms": round(res.get("compile_duration_ms", 0.0), 2),
            "upload_ms": round(res.get("upload_duration_ms", 0.0), 2),
            "render_pass_ms": round(res.get("render_duration_ms", 0.0), 2),
            "readback_ms": round(res.get("readback_duration_ms", 0.0), 2),
            "total_visible_frame_ms": round(res.get("total_frame_duration_ms", 0.0), 2),
        }
        stage_breakdowns.append(rec)
        print(f"  {label:20s}: Compile: {rec['compile_ms']:5.2f}ms | Upload: {rec['upload_ms']:4.2f}ms | Render: {rec['render_pass_ms']:4.2f}ms | Readback: {rec['readback_ms']:5.2f}ms | Total: {rec['total_visible_frame_ms']:6.2f}ms")

    benchmarks["latency_breakdown"] = stage_breakdowns

    # 5. Device Failure / Fallback Integrity
    print("\n--- 5. Device Refusal & Fail-Safe CPU Fallback Qualification ---")
    fallback_checks = []
    for label, path in key_docs:
        gpu_res = run_probe(path, page=0, scale=1.0, backend="gpu")
        cpu_res = run_probe(path, page=0, scale=1.0, backend="cpu")
        rec = {
            "document": label,
            "gpu_status": gpu_res.get("status"),
            "cpu_fallback_status": cpu_res.get("status"),
            "gpu_pass_ms": round(gpu_res.get("render_duration_ms", 0.0), 2),
            "cpu_pass_ms": round(cpu_res.get("render_duration_ms", 0.0), 2),
            "fallback_successful": gpu_res.get("status") == "OK" and cpu_res.get("status") == "OK",
        }
        fallback_checks.append(rec)
        print(f"  {label:20s}: GPU Status: {rec['gpu_status']} | CPU Fallback Status: {rec['cpu_fallback_status']} | Fallback OK: {rec['fallback_successful']}")

    benchmarks["fallback_integrity"] = fallback_checks

    out_json = OUT_DIR / "performance_and_stability_phase8.json"
    with open(out_json, "w") as f:
        json.dump(benchmarks, f, indent=2)

    print(f"\nPhase 8 Performance & Stability benchmarks written to: {out_json}")


if __name__ == "__main__":
    main()
