"""Issue 44 read-only gate. No process termination or service mutation."""
import json
import math
import plistlib
import re
import statistics
import subprocess
import time

POLICY = dict(settle_seconds=60, observe_seconds=60, interval_seconds=5,
              gpu_drift_pp=5, windowserver_drift_pp=10, absolute_gpu_limit=None)


def command(args):
    return subprocess.run(args, capture_output=True, timeout=10, check=True)


def processes():
    # Arguments are inspected transiently, never included in evidence or exceptions.
    text = command(['/bin/ps', '-axo', 'pid=,ppid=,comm=']).stdout.decode()
    return [(int(m[1]), int(m[2]), m[3]) for line in text.splitlines()
            if (m := re.fullmatch(r'\s*(\d+)\s+(\d+)\s+(.+)', line))]


def classify(rows, owned=()):
    counts = dict(build_or_server=0, other_aether=0, postgres=0, docker=0,
                  codex_helper=0, simulator_service=0)
    for pid, parent, exe in rows:
        if pid in owned:
            continue
        name = exe.rsplit('/', 1)[-1]
        if '/cua_node/bin/node' in exe or '/artifact_tool/' in exe or '/Codex/artifact-sessions/runtime-generations/' in exe:
            counts['codex_helper'] += 1
        elif name in {'node', 'npm', 'pnpm', 'bun', 'deno', 'rustc', 'cargo', 'swiftc',
                      'clang', 'clang++', 'xcodebuild', 'ninja', 'make', 'pytest', 'jest'}:
            counts['build_or_server'] += 1
        elif name == 'aether-desktop':
            counts['other_aether'] += 1
        elif name == 'postgres':
            counts['postgres'] += 1
        elif '/Docker.app/' in exe or name.startswith('com.docker.'):
            counts['docker'] += 1
        elif 'CoreSimulator' in exe or name == 'Simulator':
            counts['simulator_service'] += 1
    return counts


def conflicts(owned=(), services_stopped=True):
    counts = classify(processes(), owned)
    keys = ['build_or_server', 'other_aether']
    if services_stopped:
        keys += ['postgres', 'docker']
    return {k: counts[k] for k in keys}


def gpu_percent():
    data = plistlib.loads(command(['/usr/sbin/ioreg', '-r', '-c', 'AGXAccelerator', '-a']).stdout)
    values = []
    def visit(x):
        if isinstance(x, dict):
            p = x.get('PerformanceStatistics', {})
            if 'Device Utilization %' in p:
                values.append(p['Device Utilization %'])
            for v in x.values():
                visit(v)
        elif isinstance(x, list):
            for v in x:
                visit(v)
    visit(data)
    if len(values) != 1 or type(values[0]) not in (int, float) or not 0 <= values[0] <= 100:
        raise ValueError('GPU_UNAVAILABLE')
    return values[0]


def window_percent():
    text = command(['/bin/ps', '-axo', '%cpu=,comm=']).stdout.decode()
    values = [float(m[1]) for line in text.splitlines()
              if (m := re.fullmatch(r'\s*([\d.]+)\s+.*?/WindowServer', line))]
    if len(values) != 1:
        raise ValueError('WINDOWSERVER_UNAVAILABLE')
    return values[0]


def observation(owned=()):
    power = command(['/usr/bin/pmset', '-g', 'batt']).stdout.decode()
    return dict(monotonic_ns=time.monotonic_ns(), gpu_percent=gpu_percent(),
                windowserver_cpu_percent=window_percent(), ac="'AC Power'" in power,
                conflicts=conflicts(owned))


def evaluate(samples):
    reasons = []
    if len(samples) != 12:
        return dict(status='BLOCKED', reasons=['BASELINE_SAMPLE_COUNT'])
    for row in samples:
        if not row['ac']:
            reasons.append('AC_REQUIRED')
        if any(row['conflicts'].values()):
            reasons.append('EXTERNAL_PROCESS')
        for k in ('gpu_percent', 'windowserver_cpu_percent'):
            if type(row[k]) not in (int, float) or not math.isfinite(row[k]) or row[k] < 0:
                return dict(status='BLOCKED', reasons=['BASELINE_METRIC_UNAVAILABLE'])
    deltas = {}
    for key, threshold in [('gpu_percent', 5), ('windowserver_cpu_percent', 10)]:
        first = statistics.median(s[key] for s in samples[:6])
        last = statistics.median(s[key] for s in samples[6:])
        deltas[key] = dict(first_median=first, last_median=last, drift_pp=abs(last-first))
        if abs(last-first) > threshold:
            reasons.append(key.upper()+'_DRIFT')
    elapsed = [(s['monotonic_ns']-samples[0]['monotonic_ns'])/1e9 for s in samples]
    if any(abs(t-i*5) > 1 for i, t in enumerate(elapsed)):
        reasons.append('BASELINE_SAMPLE_TIMING')
    return dict(status='BLOCKED' if reasons else 'PASS', reasons=sorted(set(reasons)), metrics=deltas)


def baseline(path):
    # Exclusive reservation survives interruption; never silently repeat a failed gate.
    with path.open('x') as f:
        json.dump(dict(status='IN_PROGRESS', policy=POLICY), f)
    samples = []
    try:
        time.sleep(60)
        start = time.monotonic()
        for i in range(12):
            time.sleep(max(0, start+i*5-time.monotonic()))
            samples.append(observation())
        time.sleep(max(0, start+60-time.monotonic()))
        result = dict(evaluate(samples), samples=samples, policy=POLICY)
    except (ValueError, OSError, subprocess.SubprocessError):
        result = dict(status='BLOCKED', reasons=['BASELINE_READ_FAILED'], samples=samples, policy=POLICY)
    path.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k!='samples'}), flush=True)
    return result
