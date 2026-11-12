#!/usr/bin/env python3
"""E5a: one operator-driven packaged smoke, never a benchmark ranking or shell runner."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import plistlib
import queue
import re
import subprocess
import threading
import time
import uuid

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'target/e5a-issue-25'
APP = OUT / 'release/bundle/macos/AETHER.app'
PREFIX = 'AETHER_MEASURE '
VARIANTS = {'webview-a', 'node-b'}
EVENTS = {'host_main', 'session_ready', 'host_accept', 'event_accepted', 'cancel_confirmed',
          'child_spawn', 'child_ready', 'child_reaped', 'run_final'}
FIELDS = set('schema_version trial_id variant component app_instance process_pid run_id request_id generation seq event_type monotonic_ns operation_ns outcome exit_code child_pid progress terminal_count result fixture'.split())
RESULT_FIELDS = set('trial_id variant source_sha artifact_hash fixture environment_id measurement value unit status failure_reason'.split())


def command(args):
    return subprocess.run(args, cwd=ROOT, capture_output=True, text=True, timeout=10)


def sha(path):
    h = hashlib.sha256()
    with path.open('rb') as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b''):
            h.update(block)
    return h.hexdigest()


def sources():
    r = command(['git', 'ls-files', '--cached', '--others', '--exclude-standard', 'apps/desktop'])
    if r.returncode:
        raise ValueError('SOURCE_LIST_FAILED')
    return {p: sha(ROOT / p) for p in sorted(set(r.stdout.splitlines())) if (ROOT / p).is_file()}


def artifacts():
    if not APP.is_dir() or APP.is_symlink():
        raise ValueError('APP_NOT_BUILT')
    info = plistlib.loads((APP / 'Contents/Info.plist').read_bytes())
    if info.get('CFBundleExecutable') != 'aether-desktop' or info.get('CFBundleIdentifier') != 'com.aether.firstwindow':
        raise ValueError('UNEXPECTED_APP')
    executable = APP / 'Contents/MacOS/aether-desktop'
    if 'arm64' not in command(['/usr/bin/file', str(executable)]).stdout:
        raise ValueError('APP_NOT_ARM64')
    entries = {}
    size = 0
    for p in sorted(APP.rglob('*')):
        if p.is_symlink():
            raise ValueError('APP_SYMLINK_NOT_SUPPORTED')
        if p.is_file():
            entries[str(p.relative_to(APP))] = sha(p)
            size += p.stat().st_size
    pin = json.loads((ROOT / 'apps/desktop/sidecar/node-provenance.json').read_text())
    if entries['Contents/Resources/sidecar/node'] != pin['node_sha256'] or entries['Contents/Resources/sidecar/LICENSE'] != pin['license_sha256']:
        raise ValueError('NODE_PIN_MISMATCH')
    return {'artifact_hash': hashlib.sha256(json.dumps(entries, sort_keys=True).encode()).hexdigest(),
            'executable_hash': sha(executable), 'bundle_files': entries, 'app_bytes': size,
            'node_bytes': (APP / 'Contents/Resources/sidecar/node').stat().st_size,
            'node_version': pin['version']}


def uint(value, maximum=2**64-1):
    return type(value) is int and 0 <= value <= maximum


def validate_event(e):
    if not isinstance(e, dict) or set(e) != FIELDS:
        raise ValueError('EVENT_FIELDS')
    if e['schema_version'] != 1 or type(e['schema_version']) is not int:
        raise ValueError('EVENT_VERSION')
    for name in ('trial_id', 'app_instance'):
        if not isinstance(e[name], str) or not re.fullmatch('[0-9a-f]{32}', e[name]):
            raise ValueError('EVENT_ID')
    if any(not isinstance(e[k], str) for k in ('variant','component','event_type','outcome','result','fixture')):
        raise ValueError('EVENT_TEXT')
    if e['variant'] not in VARIANTS or e['component'] != 'host' or e['event_type'] not in EVENTS:
        raise ValueError('EVENT_ENUM')
    for name in ('process_pid', 'run_id', 'request_id', 'generation', 'seq', 'monotonic_ns', 'progress', 'terminal_count'):
        if not uint(e[name]):
            raise ValueError('EVENT_NUMBER')
    if e['process_pid'] == 0 or e['progress'] not in (0, 25, 50, 75) or e['terminal_count'] not in (0, 1):
        raise ValueError('EVENT_RANGE')
    for name in ('operation_ns', 'child_pid'):
        if e[name] is not None and not uint(e[name]):
            raise ValueError('EVENT_OPTIONAL_NUMBER')
    if e['exit_code'] is not None and (type(e['exit_code']) is not int or not -255 <= e['exit_code'] <= 255):
        raise ValueError('EVENT_EXIT')
    if e['outcome'] not in ('none', 'created', 'running', 'succeeded', 'failed', 'cancel_requested', 'cancelled') or e['result'] not in ('', 'FAKE_OK') or e['fixture'] not in ('none', 'normal', 'pause25', 'long60'):
        raise ValueError('EVENT_STATE')
    if e['event_type'] in ('host_accept','event_accepted','cancel_confirmed') and e['operation_ns'] is None:
        raise ValueError('MISSING_OPERATION_TIME')
    return e


def validate_result(row):
    if not isinstance(row, dict) or set(row) != RESULT_FIELDS:
        raise ValueError('RESULT_FIELDS')
    if any(not isinstance(row[k], str) for k in RESULT_FIELDS - {'value','failure_reason'}):
        raise ValueError('RESULT_TEXT')
    if row['variant'] not in VARIANTS:
        raise ValueError('RESULT_FIELDS')
    for name, length in [('trial_id', 32), ('source_sha', 40), ('artifact_hash', 64), ('environment_id', 32)]:
        if not re.fullmatch('[0-9a-f]{%d}' % length, row[name]):
            raise ValueError('RESULT_ID')
    if row['fixture'] not in ('normal', 'pause25') or row['unit'] not in ('ms', 'bytes', 'percent', 'count'):
        raise ValueError('RESULT_ENUM')
    if not re.fullmatch('[a-z_]{1,64}', row['measurement']):
        raise ValueError('RESULT_MEASUREMENT')
    if row['status'] not in ('PASS', 'FAIL', 'TIMEOUT', 'NOT_RUN', 'UNAVAILABLE', 'BLOCKED'):
        raise ValueError('RESULT_STATUS')
    if row['status'] == 'PASS':
        if type(row['value']) not in (int, float) or not math.isfinite(row['value']) or row['value'] < 0 or row['failure_reason'] is not None:
            raise ValueError('RESULT_VALUE')
    elif row['value'] is not None or not isinstance(row['failure_reason'], str) or not re.fullmatch('[A-Z0-9_]{1,80}', row['failure_reason']):
        raise ValueError('RESULT_MISSING')
    return row


def summarize(rows):
    groups = {}
    for row in rows:
        validate_result(row)
        key = (row['variant'], row['measurement'], row['unit'])
        groups.setdefault(key, []).append(row)
    result = []
    for (variant, measurement, unit), group in sorted(groups.items()):
        values = sorted(r['value'] for r in group if r['status'] == 'PASS')
        n = len(values)
        result.append({'variant': variant, 'measurement': measurement, 'unit': unit, 'n': n,
                       'total': len(group), 'failures': sum(r['status'] == 'FAIL' for r in group),
                       'timeouts': sum(r['status'] == 'TIMEOUT' for r in group),
                       'uncollected': sum(r['status'] in ('NOT_RUN', 'UNAVAILABLE', 'BLOCKED') for r in group),
                       'min': min(values) if n else None, 'max': max(values) if n else None,
                       'p50': values[math.ceil(.50*n)-1] if n else None,
                       'p95': values[math.ceil(.95*n)-1] if n else None})
    return result


def check_trace(events, trial, variant, fixture, host_pid):
    if not 1 <= len(events) <= 128:
        raise ValueError('EVENT_COUNT')
    instance = events[0]['app_instance']
    previous = -1
    for e in events:
        validate_event(e)
        if (e['trial_id'], e['variant'], e['process_pid'], e['app_instance']) != (trial, variant, host_pid, instance):
            raise ValueError('TRACE_IDENTITY')
        if e['monotonic_ns'] < previous:
            raise ValueError('TRACE_CLOCK')
        previous = e['monotonic_ns']
    accepts = [e for e in events if e['event_type'] == 'host_accept']
    if len(accepts) != 1:
        raise ValueError('ONE_RUN_REQUIRED')
    accepted = accepts[0]
    identity = tuple(accepted[k] for k in ('run_id', 'request_id', 'generation'))
    if not all(identity) or accepted['fixture'] != fixture:
        raise ValueError('FIXTURE_OR_ID')
    for e in events:
        if e['run_id'] and (tuple(e[k] for k in ('run_id', 'request_id', 'generation')) != identity or e['fixture'] != fixture):
            raise ValueError('RUN_IDENTITY')
    frames = [e for e in events if e['event_type'] == 'event_accepted']
    if [e['seq'] for e in frames] != list(range(1, len(frames)+1)):
        raise ValueError('FRAME_SEQUENCE')
    final = [e for e in events if e['event_type'] == 'run_final'] if variant == 'node-b' else [e for e in events if e['event_type'] in ('event_accepted', 'cancel_confirmed') and e['terminal_count'] == 1]
    if len(final) != 1 or final[0]['terminal_count'] != 1:
        raise ValueError('TERMINAL_COUNT')
    final = final[0]
    if fixture == 'normal':
        if [e['progress'] for e in frames] != [0,25,50,75,75] or [e['terminal_count'] for e in frames] != [0,0,0,0,1] or final['outcome'] != 'succeeded' or final['result'] != 'FAKE_OK':
            raise ValueError('NORMAL_TRACE')
    elif [e['progress'] for e in frames] != [0,25] or final['outcome'] != 'cancelled' or final['result'] != '' or len([e for e in events if e['event_type'] == 'cancel_confirmed']) != 1:
        raise ValueError('CANCEL_TRACE')
    if variant == 'node-b':
        children = [[e for e in events if e['event_type'] == kind] for kind in ('child_spawn', 'child_ready', 'child_reaped')]
        if any(len(group) != 1 for group in children) or len({g[0]['child_pid'] for g in children}) != 1 or not children[0][0]['child_pid']:
            raise ValueError('CHILD_LIFECYCLE')
        if fixture == 'normal' and children[-1][0]['exit_code'] != 0:
            raise ValueError('CHILD_EXIT')
    return accepted, final, frames


def identity(pid):
    # Private, transient OS identity. Never serialize command paths or OS start strings.
    r = command(['/bin/ps', '-p', str(pid), '-o', 'ppid=', '-o', 'lstart=', '-o', 'comm='])
    parts = r.stdout.strip().split(None, 6)
    return (int(parts[0]), ' '.join(parts[1:6]), parts[6]) if r.returncode == 0 and len(parts) == 7 else None


def cpu_seconds(text):
    days = 0
    if '-' in text:
        day, text = text.split('-', 1); days = int(day)
    parts = [float(p) for p in text.split(':')]
    if len(parts) not in (2, 3):
        raise ValueError('CPU_TIME_FORMAT')
    return days*86400 + sum(v * 60**i for i, v in enumerate(reversed(parts)))


def sample(pid, expected):
    if identity(pid) != expected:
        return None
    r = command(['/bin/ps', '-p', str(pid), '-o', 'time=', '-o', 'rss='])
    parts = r.stdout.split()
    if r.returncode or len(parts) != 2:
        return None
    return {'pid': pid, 'harness_ns': time.monotonic_ns(), 'cpu_seconds': cpu_seconds(parts[0]), 'rss_bytes': int(parts[1])*1024}


def environment():
    batt = command(['/usr/bin/pmset', '-g', 'batt'])
    text = batt.stdout
    percentage = re.search(r'(\d+)%', text)
    therm = command(['/usr/bin/pmset', '-g', 'therm'])
    return {'environment_id': uuid.uuid4().hex, 'macos': command(['/usr/bin/sw_vers','-productVersion']).stdout.strip(),
            'macos_build': command(['/usr/bin/sw_vers','-buildVersion']).stdout.strip(),
            'architecture': platform.machine(), 'host_version': 'aether-desktop 0.1.0',
            'power_source': 'AC' if 'AC Power' in text else 'BATTERY' if 'Battery Power' in text else 'UNKNOWN',
            'battery_percent': int(percentage[1]) if percentage else None,
            'battery_state': 'discharging' if 'discharging' in text else 'charging' if 'charging' in text else 'UNKNOWN',
            'thermal': 'UNAVAILABLE' if therm.returncode or 'Error' in therm.stdout+therm.stderr else 'NO_WARNING_REPORTED',
            'load_average': list(os.getloadavg()), 'logical_cpu_count': os.cpu_count(),
            'screen_condition': 'OPERATOR_FOREGROUND_NO_SYSTEM_CHANGE',
            'webkit_attribution': 'UNAVAILABLE', 'gpu_energy_wakeups': 'NOT_RUN'}


def save(path, data):
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False, allow_nan=False)+'\n')


def smoke(args, manifest, artifact):
    trial = uuid.uuid4().hex
    directory = OUT / 'results' / trial
    directory.mkdir(parents=True)
    env_info = environment()
    context = dict(trial_id=trial, variant=args.variant, source_sha=manifest['source_sha'],
                   artifact_hash=artifact['artifact_hash'], fixture=args.fixture,
                   environment_id=env_info['environment_id'])
    save(directory/'manifest.json', {**manifest, **artifact, 'environment': env_info,
                                    'trial_id': trial, 'variant': args.variant, 'fixture': args.fixture,
                                    'classification': 'SMOKE_ONLY', 'instrumentation': True,
                                    'resource_interval_ms': 250, 'timeout_seconds': 90})
    executable = APP/'Contents/MacOS/aether-desktop'
    child_path = str(APP/'Contents/Resources/sidecar/node')
    launch_ns = time.monotonic_ns()
    proc = subprocess.Popen([str(executable), '--benchmark-smoke'], cwd=directory,
                            env={'PATH': '/usr/bin:/bin', 'AETHER_BENCH_TRIAL': trial, 'AETHER_BENCH_VARIANT': args.variant},
                            stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
    owned = identity(proc.pid)
    if not owned or owned[2] != str(executable):
        # Popen handle still owns this child; no arbitrary PID is accepted.
        proc.terminate(); proc.wait(timeout=5)
        raise ValueError('HOST_IDENTITY')
    q = queue.Queue(maxsize=256)
    overflow = threading.Event()
    def reader():
        while True:
            raw = proc.stderr.readline(4097)
            if not raw:
                return
            if len(raw) > 4096:
                overflow.set(); continue
            if raw.startswith(PREFIX.encode()):
                try:
                    q.put_nowait(raw[len(PREFIX):].decode('utf-8'))
                except (queue.Full, UnicodeDecodeError):
                    overflow.set()
    thread = threading.Thread(target=reader, daemon=True); thread.start()
    events, samples, children = [], [], {}
    first_diagnostic_ns = None
    error = None
    start = time.monotonic()
    last_sample = 0.0
    print(json.dumps({'trial_id': trial, 'pid': proc.pid, 'variant': args.variant, 'fixture': args.fixture,
                      'instruction': 'CLICK requested runtime once; cancel at 25 for pause25; inspect UI then Cmd+Q'}, ensure_ascii=False), flush=True)
    try:
        while proc.poll() is None or not q.empty() or thread.is_alive():
            if time.monotonic()-start > 90:
                error = 'TIMEOUT'; break
            if overflow.is_set():
                raise ValueError('DIAGNOSTIC_LIMIT')
            try:
                e = validate_event(json.loads(q.get(timeout=.05)))
                if e['trial_id'] != trial or e['process_pid'] != proc.pid or e['variant'] != args.variant:
                    raise ValueError('TRACE_IDENTITY')
                if first_diagnostic_ns is None:
                    first_diagnostic_ns = time.monotonic_ns()
                events.append(e)
                if len(events) > 128:
                    raise ValueError('EVENT_COUNT')
                if e['event_type'] == 'child_spawn':
                    ident = identity(e['child_pid'])
                    if ident and ident[0] == proc.pid and ident[2] == child_path:
                        children[e['child_pid']] = ident
                    # Very short children may exit before sampling; never invent metrics.
                print(json.dumps({k:e[k] for k in ('event_type','run_id','seq','progress','outcome','terminal_count') }), flush=True)
            except queue.Empty:
                pass
            if time.monotonic()-last_sample >= .25:
                last_sample = time.monotonic()
                for pid, ident in [(proc.pid,owned), *children.items()]:
                    value = sample(pid, ident)
                    if value:
                        value['component'] = 'host' if pid == proc.pid else 'node'
                        samples.append(value)
        if proc.poll() is None:
            raise ValueError('TIMEOUT')
        if proc.returncode != 0:
            raise ValueError('HOST_EXIT')
        accepted, final, frames = check_trace(events, trial, args.variant, args.fixture, proc.pid)
    except (ValueError, json.JSONDecodeError) as exc:
        error = str(exc) if re.fullmatch('[A-Z_]+', str(exc)) else 'INVALID_DIAGNOSTIC'
    finally:
        if proc.poll() is None:
            proc.terminate()
            try: proc.wait(timeout=5)
            except subprocess.TimeoutExpired: proc.kill(); proc.wait(timeout=5)
        thread.join(timeout=2)
        # Host termination closes private stdin: give its child a bounded EOF exit interval.
        deadline = time.monotonic()+5
        while any(identity(pid) == ident for pid,ident in children.items()) and time.monotonic() < deadline:
            time.sleep(.1)
        for pid, ident in children.items():
            if identity(pid) == ident:
                error = 'CHILD_REMAINED'
                os.kill(pid, 9)  # exact spawned child identity, never a CLI PID
        proc.stderr.close()
    (directory/'events.jsonl').write_text(''.join(json.dumps(e)+'\n' for e in events))
    (directory/'resources.jsonl').write_text(''.join(json.dumps(s)+'\n' for s in samples))
    rows = []
    def row(name, value, unit='ms', status='PASS', reason=None):
        rows.append(validate_result({**context, 'measurement':name, 'value':value, 'unit':unit,
                                     'status':status, 'failure_reason':reason}))
    if error:
        row('trial_valid', None, 'count', 'TIMEOUT' if error == 'TIMEOUT' else 'FAIL', error)
    else:
        row('trial_valid', 1, 'count')
        row('spawn_to_first_diagnostic_observed', (first_diagnostic_ns-launch_ns)/1e6)
        ready = [e for e in events if e['event_type']=='session_ready']
        main = [e for e in events if e['event_type']=='host_main']
        if ready and main:
            row('main_to_session_ready', (ready[0]['monotonic_ns']-main[0]['monotonic_ns'])/1e6)
        row('host_accept_processing', accepted['operation_ns']/1e6)
        first = next(e for e in frames if e['progress']==25)
        row('accept_to_first_progress', (first['monotonic_ns']-accepted['monotonic_ns'])/1e6)
        row('accept_to_terminal', (final['monotonic_ns']-accepted['monotonic_ns'])/1e6)
        for e in frames:
            row('host_event_processing', e['operation_ns']/1e6)
        if args.variant == 'node-b':
            spawn = next(e for e in events if e['event_type']=='child_spawn')
            ready = next(e for e in events if e['event_type']=='child_ready')
            row('accept_to_node_ready', (ready['monotonic_ns']-accepted['monotonic_ns'])/1e6)
            row('node_spawn_to_ready', (ready['monotonic_ns']-spawn['monotonic_ns'])/1e6)
        if args.fixture == 'pause25':
            cancel = next(e for e in events if e['event_type']=='cancel_confirmed')
            row('host_cancel_processing', cancel['operation_ns']/1e6)
            if args.variant == 'node-b':
                reap = next(e for e in events if e['event_type']=='child_reaped')
                row('cancel_confirmed_to_reap', (reap['monotonic_ns']-cancel['monotonic_ns'])/1e6)
    for component in ('host', 'node'):
        values = [s for s in samples if s['component']==component]
        row(component+'_rss_peak_observed', max(s['rss_bytes'] for s in values) if values else None,
            'bytes', 'PASS' if values else 'UNAVAILABLE', None if values else 'NO_PROCESS_SAMPLE')
        if len(values)>=2:
            cpu = 100*(values[-1]['cpu_seconds']-values[0]['cpu_seconds'])/((values[-1]['harness_ns']-values[0]['harness_ns'])/1e9)
            row(component+'_cpu_observed_interval', cpu, 'percent')
        else:
            row(component+'_cpu_observed_interval',None,'percent','UNAVAILABLE','INSUFFICIENT_SAMPLES')
    for name in ('first_window_paint','cold_launch','webview_cpu','webview_rss','gpu','energy','idle_wakeups','ui_request_to_host','provider_to_host_latency'):
        row(name,None,'count','NOT_RUN','NO_VALIDATED_MEASUREMENT')
    row('app_size',artifact['app_bytes'],'bytes'); row('bundled_node_size',artifact['node_bytes'],'bytes')
    save(directory/'results.json',rows)
    summary = {'classification':'SMOKE_ONLY', 'status':'PASS' if not error else 'TIMEOUT' if error=='TIMEOUT' else 'FAIL',
               'host_exit_code':proc.returncode,'failure_reason':error,'gui':'REQUIRES_SEPARATE_OBSERVATION',
               'results':summarize(rows)}
    save(directory/'summary.json',summary)
    print(json.dumps({'trial_id':trial,'status':summary['status'],'failure_reason':error,'host_exit_code':proc.returncode}),flush=True)
    return 0 if not error else 1


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    group = parser.add_mutually_exclusive_group(required=True)
    group.add_argument('--dry-run', action='store_true')
    group.add_argument('--seal-build', action='store_true', help='After build only: tie bundle hashes to current app source inputs')
    group.add_argument('--smoke', action='store_true')
    parser.add_argument('--variant', choices=sorted(VARIANTS), default='webview-a')
    parser.add_argument('--fixture', choices=['normal','pause25'],default='normal')
    args = parser.parse_args()
    if platform.system() != 'Darwin' or platform.machine() != 'arm64':
        raise ValueError('APPLE_SILICON_REQUIRED')
    artifact = artifacts()
    head = command(['git','rev-parse','HEAD']).stdout.strip()
    inputs = sources()
    seal_path = OUT/'build-manifest.json'
    if args.seal_build:
        save(seal_path, {'source_sha':head,'source_inputs':inputs, **artifact})
        print('PASS: sealed current build inputs; build provenance requires the recorded build command')
        return 0
    seal = json.loads(seal_path.read_text())
    if seal['source_inputs'] != inputs or seal['artifact_hash'] != artifact['artifact_hash']:
        raise ValueError('SOURCE_OR_ARTIFACT_CHANGED')
    manifest = {'source_sha':head,'source_inputs':inputs,'build_source_sha':seal['source_sha'],
                'source_worktree_dirty':bool(command(['git','status','--porcelain','--','apps/desktop']).stdout),
                'fixture_hash':sha(ROOT/'apps/desktop/src/fake-provider.ts')}
    if args.dry_run:
        print(json.dumps({'status':'PASS','source_sha':head,'artifact_hash':artifact['artifact_hash'],
                          'source_worktree_dirty':manifest['source_worktree_dirty'],'variant':args.variant,'fixture':args.fixture,'app_executed':False}))
        return 0
    return smoke(args,manifest,artifact)


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (ValueError, OSError, subprocess.SubprocessError, KeyError) as exc:
        # Paths and original OS errors remain private; no raw logs in saved diagnostics.
        code = str(exc) if re.fullmatch('[A-Z_]+', str(exc)) else type(exc).__name__
        print(json.dumps({'status':'BLOCKED','failure_reason':code}))
        raise SystemExit(2)
