#!/usr/bin/env python3
"""Isolated StarPDF corpus qualification. Outputs are never committed."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import time
import threading

ROOT = Path(__file__).resolve().parents[2]
MANIFEST = Path(__file__).with_name('corpus.json')
CLASSES = {'STARPDF_FAILURE', 'ORACLE_DISAGREEMENT', 'UNSUPPORTED', 'INVALID_FIXTURE', 'UNKNOWN'}


def run(command, timeout=30, cwd=ROOT):
    start = time.perf_counter()
    try:
        import psutil
    except ImportError:
        psutil = None
    peak = [None]
    done = threading.Event()
    try:
        proc = subprocess.Popen(command, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        def sample():
            if psutil is None: return
            try:
                process = psutil.Process(proc.pid)
                while not done.is_set():
                    rss = sum(p.memory_info().rss for p in [process] + process.children(recursive=True) if p.is_running())
                    peak[0] = max(peak[0] or 0, rss)
                    done.wait(0.01)
            except psutil.Error:
                pass
        sampler = threading.Thread(target=sample, daemon=True)
        sampler.start()
        try:
            stdout, stderr = proc.communicate(timeout=timeout)
            timed_out = False
        except subprocess.TimeoutExpired:
            proc.kill()
            stdout, stderr = proc.communicate()
            timed_out = True
        finally:
            done.set()
            sampler.join(timeout=0.1)
        return {'ok': proc.returncode == 0 and not timed_out, 'exit_code': proc.returncode, 'duration_ms': round((time.perf_counter() - start) * 1000, 3), 'stdout': stdout, 'stderr': stderr[-4000:], 'crash': proc.returncode < 0 and not timed_out, 'timeout': timed_out, 'peak_rss_bytes': peak[0]}
    except subprocess.TimeoutExpired as exc:
        return {'ok': False, 'exit_code': None, 'duration_ms': round((time.perf_counter() - start) * 1000, 3), 'stdout': str(exc.stdout or '')[-1000:], 'stderr': str(exc.stderr or '')[-1000:], 'crash': False, 'timeout': True, 'peak_rss_bytes': None}


def failure(item, classification, stage, detail):
    assert classification in CLASSES
    item['failures'].append({'classification': classification, 'stage': stage, 'detail': str(detail)})


def render(pdf, prefix):
    if not shutil.which('pdftoppm'):
        return {'status': 'UNSUPPORTED', 'reason': 'Poppler pdftoppm unavailable'}
    result = run(['pdftoppm', '-f', '1', '-l', '1', '-scale-to', '900', '-singlefile', '-png', str(pdf), str(prefix)], timeout=30)
    return {'status': 'SUCCESS' if result['ok'] else 'FAILURE', 'process': result, 'path': str(prefix) + '.png' if result['ok'] else None}


def pixels(before, after):
    try:
        from PIL import Image, ImageChops
        a, b = Image.open(before).convert('RGB'), Image.open(after).convert('RGB')
        if a.size != b.size:
            return {'size_before': a.size, 'size_after': b.size, 'different_pixels': None, 'reason': 'dimensions differ'}
        diff = ImageChops.difference(a, b)
        return {'size': a.size, 'different_pixels': sum(1 for pixel in diff.getdata() if pixel != (0, 0, 0)), 'total_pixels': a.width * a.height}
    except ImportError:
        return {'different_pixels': None, 'reason': 'Pillow unavailable'}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--out', type=Path, default=ROOT / 'qualification/starpdf/out')
    parser.add_argument('--limit', type=int)
    parser.add_argument('--regressions', action='store_true')
    args = parser.parse_args()
    manifest = json.loads(MANIFEST.read_text())
    args.out.mkdir(parents=True, exist_ok=True)
    probe = ROOT / 'engine/starpdf/target/debug/examples/qualification_probe'
    build = run(['cargo', 'build', '--example', 'qualification_probe'], 180, ROOT / 'engine/starpdf')
    report = {'schema_version': 1, 'base': manifest['base'], 'branch': 'arch/qualification', 'tools': {name: shutil.which(name) for name in ('pdftoppm', 'pdfinfo', 'pdftotext', 'qpdf', 'node')}, 'build': build, 'fixtures': [], 'regressions': [], 'performance_baseline': {}, 'failures': []}
    if not build['ok']:
        report['failures'].append({'classification': 'STARPDF_FAILURE', 'stage': 'BUILD', 'detail': build['stderr']})
    else:
        pdfjs = ROOT.parent / 'toolrakyat/node_modules/pdfjs-dist/legacy/build/pdf.mjs'
        for fixture in manifest['fixtures'][:args.limit]:
            item = {'id': fixture['id'], 'categories': fixture['categories'], 'stages': {}, 'oracles': {}, 'failures': [], 'mutation': None}
            report['fixtures'].append(item)
            source = ROOT / fixture['path']
            if not source.is_file() or hashlib.sha256(source.read_bytes()).hexdigest() != fixture['sha256']:
                failure(item, 'INVALID_FIXTURE', 'PROVENANCE', 'missing file or SHA-256 mismatch')
                continue
            item['stages']['PROVENANCE'] = {'status': 'SUCCESS', 'sha256': fixture['sha256'], 'source': fixture['provenance']}
            with tempfile.TemporaryDirectory(prefix='starpdf-qualification-') as tmp:
                tmp = Path(tmp)
                exported = tmp / 'export.pdf'
                probe_run = run([str(probe), str(source)] + ([str(exported)] if fixture['edit'] else []), 30)
                item['stages']['OPEN_PARSE_TEXT'] = {key: value for key, value in probe_run.items() if key != 'stdout'}
                if not probe_run['ok']:
                    failure(item, 'STARPDF_FAILURE', 'OPEN_PARSE_TEXT', probe_run['stderr'] or 'crash/timeout')
                    continue
                try:
                    data = json.loads(probe_run['stdout'])
                except json.JSONDecodeError:
                    failure(item, 'UNKNOWN', 'OPEN_PARSE_TEXT', 'probe did not return JSON')
                    continue
                item['starpdf'] = data
                for stage, key in (('OPEN', 'open'), ('PARSE', 'parse'), ('FIRST_PAGE', 'first_page'), ('TEXT_EXTRACT', 'text')):
                    item['stages'][stage] = {'status': 'SUCCESS' if key in data.get('duration_us', {}) and data.get('stage') != stage else 'FAILURE', 'duration_us': data.get('duration_us', {}).get(key), 'peak_rss_bytes': probe_run['peak_rss_bytes']}
                item['stages']['HIT_TEST'] = {'status': 'UNSUPPORTED', 'reason': 'no production hit-test renderer exposed at base'}
                item['stages']['STARPDF_RENDER'] = {'status': 'UNSUPPORTED', 'reason': 'no production raster renderer exposed at base'}
                if data.get('error'):
                    classification = 'UNSUPPORTED' if 'encryp' in str(data['error']).lower() else 'STARPDF_FAILURE'
                    failure(item, classification, data.get('stage', 'OPEN'), data['error'])
                if 'encryption' in fixture['categories'] and 'encrypted' in fixture['id']:
                    item['stages']['RENDER'] = {'status': 'UNSUPPORTED', 'reason': 'synthetic encrypted placeholder has no usable password or recipient'}
                else:
                    item['stages']['RENDER'] = render(source, tmp / 'before')
                    item['stages']['RENDER']['engine'] = 'Poppler oracle'
                if item['stages']['RENDER']['status'] == 'FAILURE':
                    failure(item, 'ORACLE_DISAGREEMENT', 'RENDER', item['stages']['RENDER']['process']['stderr'])
                if shutil.which('pdfinfo'):
                    info = run(['pdfinfo', str(source)])
                    item['oracles']['poppler_info'] = info
                    if info['ok']:
                        page_line = next((line for line in info['stdout'].splitlines() if line.startswith('Pages:')), None)
                        if page_line and data.get('page_count') is not None and int(page_line.split(':')[1]) != data['page_count']:
                            failure(item, 'ORACLE_DISAGREEMENT', 'PARSE', 'Poppler page count differs')
                    elif 'encrypted' not in fixture['id']:
                        failure(item, 'ORACLE_DISAGREEMENT', 'POPPLER_INFO', info['stderr'])
                if shutil.which('qpdf'):
                    item['oracles']['qpdf'] = run(['qpdf', '--check', str(source)])
                    if not item['oracles']['qpdf']['ok'] and 'encrypted' not in fixture['id'] and 'malformed' not in fixture['id']:
                        failure(item, 'ORACLE_DISAGREEMENT', 'QPDF', item['oracles']['qpdf']['stderr'])
                else:
                    item['oracles']['qpdf'] = {'status': 'UNSUPPORTED', 'reason': 'qpdf unavailable'}
                if shutil.which('node') and pdfjs.is_file():
                    js = run(['node', str(Path(__file__).with_name('pdfjs_oracle.mjs')), str(pdfjs), str(source)], 30)
                    item['oracles']['pdfjs'] = {key: value for key, value in js.items() if key != 'stdout'}
                    if js['ok']:
                        try:
                            oracle = json.loads(js['stdout'])
                            item['oracles']['pdfjs']['result'] = oracle
                            if data.get('page_count') is not None and oracle['page_count'] != data['page_count']:
                                failure(item, 'ORACLE_DISAGREEMENT', 'PARSE', 'PDF.js page count differs')
                        except (ValueError, KeyError):
                            failure(item, 'UNKNOWN', 'PDFJS', 'invalid oracle JSON')
                    elif 'encrypted' not in fixture['id'] and 'malformed' not in fixture['id']:
                        failure(item, 'ORACLE_DISAGREEMENT', 'PDFJS', js['stderr'])
                else:
                    item['oracles']['pdfjs'] = {'status': 'UNSUPPORTED', 'reason': 'local PDF.js unavailable'}
                if fixture['edit']:
                    edit = data.get('edit', {})
                    item['stages']['EDIT_EXPORT_REOPEN'] = edit
                    if edit.get('status') == 'SUCCESS':
                        if not exported.is_file() or edit.get('reopened_page_count') != data.get('page_count') or edit.get('expected_after') not in str(edit.get('actual_after')):
                            failure(item, 'STARPDF_FAILURE', 'REOPEN', 'export or form value mismatch')
                        item['stages']['RENDER_AGAIN'] = render(exported, tmp / 'after')
                        item['stages']['RENDER_AGAIN']['engine'] = 'Poppler oracle'
                        if item['stages']['RENDER_AGAIN']['status'] == 'SUCCESS' and item['stages']['RENDER']['status'] == 'SUCCESS':
                            item['pixel_difference'] = pixels(tmp / 'before.png', tmp / 'after.png')
                        if item['stages']['RENDER_AGAIN']['status'] == 'FAILURE':
                            failure(item, 'STARPDF_FAILURE', 'RENDER_AGAIN', item['stages']['RENDER_AGAIN']['process']['stderr'])
                        planned, actual = set(edit.get('plan_objects', [])), set(edit.get('changed_objects', []))
                        unexplained = sorted(actual - planned)
                        item['mutation'] = {'expected_mutation_envelope': {'operation': 'SetTextField', 'target': edit.get('field_ref'), 'expected_text': edit.get('expected_after'), 'allowed_objects': sorted(planned), 'authority': 'StarPDF mutation plan; provisional allowlist'}, 'actual_document_delta': edit.get('actual_document_delta'), 'mutation_certificate': edit.get('mutation_certificate'), 'unexplained_change_count': len(unexplained)}
                        if unexplained: failure(item, 'STARPDF_FAILURE', 'MUTATION_CERTIFICATE', unexplained)
                    else:
                        failure(item, 'UNSUPPORTED', 'EDIT', edit.get('reason', 'edit unsupported'))
                else:
                    for stage in ('EDIT', 'EXPORT', 'REOPEN', 'RENDER_AGAIN'):
                        item['stages'][stage] = {'status': 'UNSUPPORTED', 'reason': 'fixture has no declared controlled edit'}
        if args.regressions:
            for name in sorted({test for tests in manifest['coverage_tests'].values() for test in tests}):
                result = run(['cargo', 'test', '--test', name, '--quiet'], 360, ROOT / 'engine/starpdf')
                report['regressions'].append({'test': name, **result})
                if not result['ok']: report['failures'].append({'classification': 'STARPDF_FAILURE', 'stage': f'REGRESSION:{name}', 'detail': result['stderr']})
    measurements = {}
    for item in report['fixtures']:
        for key, value in item.get('starpdf', {}).get('duration_us', {}).items(): measurements.setdefault(key, []).append(value)
    report['performance_baseline'] = {key: {'samples': len(values), 'median_us': sorted(values)[len(values)//2], 'min_us': min(values), 'max_us': max(values)} for key, values in measurements.items()}
    report['performance_baseline']['hit_test'] = {'samples': 0, 'status': 'UNSUPPORTED'}
    report['performance_baseline']['starpdf_render'] = {'samples': 0, 'status': 'UNSUPPORTED'}
    report['performance_baseline']['ap_generation'] = {'samples': 0, 'status': 'UNSUPPORTED', 'reason': 'not separately timed from mutation planning'}
    report['category_coverage'] = {category: {'fixtures': [item['id'] for item in report['fixtures'] if category in item['categories']], 'count': sum(category in item['categories'] for item in report['fixtures'])} for category in manifest['categories']}
    report['failures'].extend({'fixture': item['id'], **issue} for item in report['fixtures'] for issue in item['failures'] if issue['classification'] not in ('UNSUPPORTED',))
    report['status'] = 'PASS' if not report['failures'] else 'FAIL'
    (args.out / 'qualification.json').write_text(json.dumps(report, indent=2) + '\n')
    counts = {class_name: sum(1 for i in report['failures'] if i['classification'] == class_name) for class_name in sorted(CLASSES)}
    lines = ['# StarPDF qualification', '', f"Status: **{report['status']}**", f"Base: `{manifest['base']}`", f"Fixtures: {len(report['fixtures'])}; controlled edits: {sum(bool(i.get('mutation')) for i in report['fixtures'])}; regressions: {len(report['regressions'])}", '', '## Failures', '']
    lines += [f'- {name}: {count}' for name, count in counts.items()]
    lines += ['', '## Performance baseline', '', '| Stage | Samples | Median µs | Range µs |', '|---|---:|---:|---:|']
    lines += [f"| {name} | {value['samples']} | {value['median_us']} | {value['min_us']}–{value['max_us']} |" for name, value in sorted(report['performance_baseline'].items()) if value['samples']]
    lines += ['', '## Coverage gaps', '']
    lines += [f"- {name}: no committed corpus fixture" for name, value in report['category_coverage'].items() if not value['count']]
    lines += ['', 'Peak RSS is sampled where psutil is available and may miss very short spikes. StarPDF raster render, hit test, and separate AP generation timings are unsupported at this base. Mutation certificates remain REFUSED until structural and visual closure can be independently established.', '', 'See `qualification.json` for every stage and failure.']
    (args.out / 'report.md').write_text('\n'.join(lines) + '\n')
    print(f"{report['status']}: {args.out / 'report.md'}")
    return 0 if report['status'] == 'PASS' else 1


if __name__ == '__main__': sys.exit(main())
