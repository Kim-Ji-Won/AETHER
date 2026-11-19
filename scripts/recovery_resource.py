#!/usr/bin/env python3
"""Issue 44 MINIMUM_SCOPE: fixed app, identity-bound Host/Node samples, operator GUI."""
import argparse
import json
import os
from pathlib import Path
import queue
import re
import subprocess
import threading
import time
import recovery_latency as latency

OUT=latency.OUT/'resource'


def prepare(plan):
    latency.verify(plan)
    summary=json.loads((latency.OUT/'summary.json').read_text())
    if not summary['measurement_complete'] or (latency.OUT/'stop.json').exists():
        raise ValueError('LATENCY_NOT_COMPLETE')
    latency.gate_observation()
    OUT.mkdir(exist_ok=False)
    sessions=[dict(index=i,variant=v,round=n+1) for n,pair in enumerate(plan['resource_followup']['order'])
              for i,v in [(2*n+j,v) for j,v in enumerate(pair)]]
    with (OUT/'plan.json').open('x') as f:
        json.dump(dict(source_sha=plan['source_sha'],artifact_hash=latency.EXPECTED_BUNDLE,
                       conditions=plan['resource_followup'],sessions=sessions),f,indent=2)
    latency.environment.baseline(OUT/'baseline.json')


def session(item,plan):
    latency.verify(plan);latency.gate_observation()
    if json.loads((OUT/'baseline.json').read_text())['status']!='PASS':raise ValueError('BASELINE_NOT_PASS')
    if (OUT/'stop.json').exists():raise ValueError('STOP_LATCH_SET')
    d=OUT/('%02d'%item['index']);d.mkdir(exist_ok=False)
    exe=str(latency.APP/'Contents/MacOS/aether-desktop')
    proc=subprocess.Popen([exe],cwd=d,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,env={'PATH':'/usr/bin:/bin'})
    ident=latency.process.identity(proc.pid)
    if not ident or ident[2]!=exe:raise ValueError('HOST_IDENTITY_UNVERIFIED_NO_TERMINATION')
    q=queue.Queue(512);overflow=threading.Event()
    def read():
        for line in iter(lambda:proc.stderr.readline(4097),b''):
            if len(line)>4096:overflow.set();continue
            try:q.put_nowait(line.decode())
            except (queue.Full,UnicodeError):overflow.set()
    reader=threading.Thread(target=read,daemon=True);reader.start()
    children={};samples=[];events=[];instance=None;problem=None;start=time.monotonic();next_sample=start
    latency.base.save(OUT/'current.json',dict(item,pid=proc.pid,status='RUNNING'))
    print(json.dumps(dict(item,pid=proc.pid)),flush=True)
    try:
        while proc.poll() is None or reader.is_alive() or not q.empty():
            if time.monotonic()-start>400:raise ValueError('TIMEOUT')
            if overflow.is_set():raise ValueError('OUTPUT_LIMIT')
            try:line=q.get(timeout=.02)
            except queue.Empty:line=''
            m=re.fullmatch(r'AETHER_INSTANCE instance=([a-f0-9]{32}) pid=(\d+) time_ms=\d+ executable=Contents/MacOS/aether-desktop\n',line)
            if m and int(m[2])==proc.pid:instance=m[1]
            m=re.search(r'AETHER_SIDECAR pid=(\d+) parent=(\d+) .*executable=Contents/Resources/sidecar/node ',line)
            if m and int(m[2])==proc.pid:
                pid=int(m[1]);child=latency.process.identity(pid)
                if child and child[0]==proc.pid and child[2]==str(latency.APP/'Contents/Resources/sidecar/node'):children[pid]=child
            # Keep only bounded nonprivate fixed-diagnostic fields, not raw logs.
            m=re.match(r'AETHER_FAKE variant=(webview-a|node-b) version=1 run_id=(\d+).*seq=(\d+) type=(\w+) value=(\w*) state=(\w+) terminal_count=(\d+)',line)
            if m:events.append(dict(variant=m[1],run_id=int(m[2]),seq=int(m[3]),type=m[4],value=m[5],state=m[6],terminal_count=int(m[7])))
            now=time.monotonic()
            if now>=next_sample:
                next_sample=now+1
                live={p:i for p,i in children.items() if latency.process.identity(p)==i}
                counts=latency.owned_conflicts(proc.pid,ident,str(latency.APP/'Contents/Resources/sidecar/node'),children)
                if any(counts.values()):raise ValueError('ENVIRONMENT_CONFLICT')
                latency.ac_gate(latency.base.environment())
                samples.append(dict(elapsed_seconds=now-start,host=latency.process.sample(proc.pid,ident),
                                    children={str(p):latency.process.sample(p,i) for p,i in live.items()}))
                if len(samples)>402:raise ValueError('SAMPLE_LIMIT')
        if proc.returncode!=0:raise ValueError('HOST_EXIT')
        if not instance:raise ValueError('INSTANCE_MISSING')
    except (ValueError,OSError,subprocess.SubprocessError) as exc:
        problem=str(exc) if re.fullmatch('[A-Z_]+',str(exc)) else type(exc).__name__
    finally:
        if proc.poll() is None and latency.process.identity(proc.pid)==ident:
            proc.terminate()
            try:proc.wait(timeout=5)
            except subprocess.TimeoutExpired:problem='HOST_REMAINED'
        if any(latency.process.identity(p)==i for p,i in children.items()):problem='CHILD_REMAINED'
        reader.join(timeout=1)
        if not reader.is_alive():proc.stderr.close()
    result=dict(item,status='BLOCKED' if problem else 'COLLECTED_GUI_REVIEW_REQUIRED',failure_reason=problem,
                instance=instance,host_pid=proc.pid,host_exit=proc.returncode,source_sha=plan['source_sha'],
                artifact_hash=latency.EXPECTED_BUNDLE,samples=samples,events=events,observed_child_count=len(children))
    latency.base.save(d/'result.json',result);latency.base.save(OUT/'current.json',dict(item,status=result['status']))
    if problem:
        with (OUT/'stop.json').open('x') as f:json.dump({'reason':problem},f)
    print(json.dumps(dict(index=item['index'],status=result['status'],samples=len(samples),failure_reason=problem)))


def main():
    p=argparse.ArgumentParser(description=__doc__);p.add_argument('action',choices=['prepare','next']);args=p.parse_args()
    latency.verify_seal();plan=json.loads((latency.OUT/'plan.json').read_text())
    with (latency.OUT/'.collector.lock').open('a') as lock:
        latency.fcntl.flock(lock,latency.fcntl.LOCK_EX|latency.fcntl.LOCK_NB)
        if args.action=='prepare':return prepare(plan)
        rp=json.loads((OUT/'plan.json').read_text())
        expected=[dict(index=2*n+j,variant=v,round=n+1) for n,pair in enumerate(plan['resource_followup']['order']) for j,v in enumerate(pair)]
        if rp['sessions']!=expected or rp['conditions']!=plan['resource_followup']:raise ValueError('RESOURCE_PLAN_CHANGED')
        for item in expected:
            if not (OUT/('%02d'%item['index'])).exists():return session(item,plan)

if __name__=='__main__':
    try:main()
    except (ValueError,OSError,subprocess.SubprocessError) as exc:
        print(json.dumps(dict(status='BLOCKED',reason=str(exc) if re.fullmatch('[A-Z_]+',str(exc)) else type(exc).__name__)))
        raise SystemExit(2)
