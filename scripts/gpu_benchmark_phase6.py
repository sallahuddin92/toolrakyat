#!/usr/bin/env python3
"""Phase 6 Comprehensive GPU Benchmark & Latency Profiler.

Measures and breaks down:
- IR Compilation latency
- CPU Rendering latency (NativeRasterizer)
- GPU Buffer & Texture Upload latency
- GPU Command Encoding & Render Pass latency
- GPU Staging Buffer Readback latency
- Total Visible-Frame Presentation latency
- Cold vs Warm execution across 20 iterations
"""

import json
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PROBE_BIN = ROOT / "target/debug/examples/render_page_probe"
OUT_DIR = ROOT / "target/corpus_differential_phase6"

BENCHMARK_FIXTURES = [
    ("vector-primitives.pdf", "Vector Graphics & Transforms (100% GPU Native)"),
    ("scanned-test.pdf", "Full-Page Scanned Raster (100% GPU Native)"),
    ("image-shared-xobject.pdf", "Image XObjects & Masking (100% GPU Native)"),
    ("flat-form-generated.pdf", "Vector Form Elements (100% GPU Native)"),
    ("smartpdf-adobe-like-form.pdf", "Complex Form Layout (Hybrid GPU/CPU)"),
    ("text-multispan-heading.pdf", "Multispan Headings & Text (Hybrid GPU/CPU)"),
]


def run_probe(pdf_path, backend="cpu", page=0, scale=1.0):
    cmd = [str(PROBE_BIN), str(pdf_path), str(page), str(scale), "/dev/null", backend]
    t0 = time.perf_counter()
    res = subprocess.run(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    wall_ms = (time.perf_counter() - t0) * 1000.0
    if res.returncode != 0:
        return {"status": "ERROR", "error": res.stderr}
    try:
        data = json.loads(res.stdout.strip())
        data["wall_total_ms"] = wall_ms
        return data
    except Exception as e:
        return {"status": "ERROR", "error": str(e)}


def benchmark_fixture(rel_name, desc, iterations=20):
    pdf_path = ROOT / "test-assets" / rel_name
    if not pdf_path.exists():
        return None

    print(f"\nBenchmarking: {rel_name} — {desc}")

    # Cold run
    cold_cpu = run_probe(pdf_path, backend="cpu")
    cold_gpu = run_probe(pdf_path, backend="gpu")

    # Warm iterations
    cpu_render_times = []
    gpu_compile_times = []
    gpu_upload_times = []
    gpu_render_times = []
    gpu_readback_times = []
    gpu_total_frame_times = []
    gpu_wall_times = []

    gpu_draw_calls = 0
    gpu_paths = 0
    gpu_images = 0
    fallback_items = 0
    fallback_reasons = []

    for _ in range(iterations):
        c = run_probe(pdf_path, backend="cpu")
        if c.get("status") == "OK":
            cpu_render_times.append(c.get("render_duration_ms", 0.0))

        g = run_probe(pdf_path, backend="gpu")
        if g.get("status") == "OK":
            gpu_compile_times.append(g.get("compile_duration_ms", 0.0))
            gpu_upload_times.append(g.get("upload_duration_ms", 0.0))
            gpu_render_times.append(g.get("render_duration_ms", 0.0))
            gpu_readback_times.append(g.get("readback_duration_ms", 0.0))
            gpu_total_frame_times.append(g.get("total_frame_duration_ms", 0.0))
            gpu_wall_times.append(g.get("wall_total_ms", 0.0))
            gpu_draw_calls = g.get("gpu_draw_calls", 0)
            gpu_paths = g.get("gpu_paths_rendered", 0)
            gpu_images = g.get("gpu_images_rendered", 0)
            fallback_items = g.get("fallback_cpu_items", 0)
            fallback_reasons = g.get("fallback_reasons", [])

    return {
        "fixture": rel_name,
        "description": desc,
        "iterations": iterations,
        "classification": "100%_GPU_NATIVE" if fallback_items == 0 else "HYBRID_GPU_CPU",
        "gpu_draw_calls": gpu_draw_calls,
        "gpu_paths_rendered": gpu_paths,
        "gpu_images_rendered": gpu_images,
        "fallback_cpu_items": fallback_items,
        "fallback_reasons": fallback_reasons,
        "cold_latencies_ms": {
            "cpu_wall": cold_cpu.get("wall_total_ms"),
            "gpu_wall": cold_gpu.get("wall_total_ms"),
            "gpu_frame": cold_gpu.get("total_frame_duration_ms"),
        },
        "warm_metrics_ms": {
            "cpu_render": {
                "mean": round(statistics.mean(cpu_render_times), 3),
                "median": round(statistics.median(cpu_render_times), 3),
                "min": round(min(cpu_render_times), 3),
                "max": round(max(cpu_render_times), 3),
            },
            "gpu_compile": {
                "mean": round(statistics.mean(gpu_compile_times), 3),
                "median": round(statistics.median(gpu_compile_times), 3),
            },
            "gpu_upload": {
                "mean": round(statistics.mean(gpu_upload_times), 3),
                "median": round(statistics.median(gpu_upload_times), 3),
            },
            "gpu_render_pass": {
                "mean": round(statistics.mean(gpu_render_times), 3),
                "median": round(statistics.median(gpu_render_times), 3),
            },
            "gpu_readback": {
                "mean": round(statistics.mean(gpu_readback_times), 3),
                "median": round(statistics.median(gpu_readback_times), 3),
            },
            "gpu_total_frame": {
                "mean": round(statistics.mean(gpu_total_frame_times), 3),
                "median": round(statistics.median(gpu_total_frame_times), 3),
                "min": round(min(gpu_total_frame_times), 3),
                "max": round(max(gpu_total_frame_times), 3),
            },
            "gpu_wall_total": {
                "mean": round(statistics.mean(gpu_wall_times), 3),
                "median": round(statistics.median(gpu_wall_times), 3),
            },
            "speedup_ratio_render": round(
                statistics.mean(cpu_render_times) / max(statistics.mean(gpu_render_times), 0.001), 2
            ),
            "speedup_ratio_frame": round(
                statistics.mean(cpu_render_times) / max(statistics.mean(gpu_total_frame_times), 0.001), 2
            ),
        },
    }


def main():
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    print("=" * 80)
    print("STARPDF PHASE 6: GPU VS CPU PERFORMANCE BENCHMARK SUITE")
    print("=" * 80)

    bench_results = []
    for rel_name, desc in BENCHMARK_FIXTURES:
        res = benchmark_fixture(rel_name, desc, iterations=15)
        if res:
            bench_results.append(res)
            w = res["warm_metrics_ms"]
            print(f"  Classification:        {res['classification']}")
            print(f"  GPU Draws:             {res['gpu_draw_calls']} (Paths: {res['gpu_paths_rendered']}, Images: {res['gpu_images_rendered']}, Fallback: {res['fallback_cpu_items']})")
            print(f"  Cold Frame Latency:    {res['cold_latencies_ms']['gpu_frame']:.2f} ms (Wall: {res['cold_latencies_ms']['gpu_wall']:.2f} ms)")
            print(f"  Warm CPU Render:       {w['cpu_render']['mean']:.2f} ms (median: {w['cpu_render']['median']:.2f} ms)")
            print(f"  Warm GPU Breakdown:    Upload: {w['gpu_upload']['mean']:.2f} ms | Render: {w['gpu_render_pass']['mean']:.2f} ms | Readback: {w['gpu_readback']['mean']:.2f} ms")
            print(f"  Warm GPU Total Frame:  {w['gpu_total_frame']['mean']:.2f} ms (min: {w['gpu_total_frame']['min']:.2f} ms, max: {w['gpu_total_frame']['max']:.2f} ms)")
            print(f"  Render Pass Speedup:   {w['speedup_ratio_render']}x")
            print(f"  Total Frame Speedup:   {w['speedup_ratio_frame']}x")

    out_file = OUT_DIR / "phase6_benchmarks.json"
    with open(out_file, "w") as f:
        json.dump(bench_results, f, indent=2)
    print("\nBenchmark results written to:", out_file)


if __name__ == "__main__":
    main()
