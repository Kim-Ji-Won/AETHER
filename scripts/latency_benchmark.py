#!/usr/bin/env python3
"""E5b fixed, operator-driven trials. No UI injection, arbitrary executable, or PID option."""
import argparse
import json
import math
import os
from pathlib import Path
import queue
import re
import subprocess
import threading
import time
import uuid
import sys
sys.dont_write_bytecode = True
import benchmark as base

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT/'target/e5b-issue-27'
APP = OUT/'release/bundle/macos/AETHER.app'
base.ROOT, base.OUT, base.APP = ROOT, OUT, APP


def schedule():
    items = []
    for phase, count in [('warmup',3),('measurement',30)]:
        for n in range(count):
            variants = ['webview-a','node-b'] if n % 2 == 0 else ['node-b','webview-a']
            for fixture in ('normal','pause25'):
                for variant in variants:
                    items.append(dict(phase=phase,round=n+1,variant=variant,fixture=fixture,instrumented=True))
    for n in range(3):
        for variant in ('webview-a','node-b'):
            for enabled in ([True,False] if n%2==0 else [False,True]):
                items.append(dict(phase='overhead',round=n+1,variant=variant,fixture='normal',instrumented=enabled))
    return [dict(index=i,trial_id=uuid.uuid4().hex,**item) for i,item in enumerate(items)]


def stats(values):
    if any(type(x) not in (int,float) or not math.isfinite(x) or x<0 for x in values):
        raise ValueError('INVALID_STAT_VALUE')
    values=sorted(values); n=len(values)
    return dict(n=n,min=values[0] if n else None,max=values[-1] if n else None,
                p50=values[math.ceil(n*.5)-1] if n else None,p95=values[math.ceil(n*.95)-1] if n else None,unit='ms')


def competing_work():
    r=base.command(['/bin/ps','-axo','pid=,comm=,args='])
    if r.returncode: raise ValueError('PROCESS_INSPECTION_UNAVAILABLE')
    own=str(OUT)
    # Read transiently only; never serialize other processes or private arguments.
    builds=0; other_apps=0
    for line in r.stdout.splitlines():
        if own in line: continue
        if re.search(r'/(?:rustc|cargo|clang|swiftc)(?:\s|$)',line) or re.search(r'node .*(?:vite|tauri).*\b(?:build|dev)\b',line): builds+=1
        if 'AETHER.app/Contents/MacOS/aether-desktop' in line: other_apps+=1
    return {'build_processes':builds,'other_aether_processes':other_apps}


def environment_key(env):
    return tuple(env[k] for k in ('macos_build','architecture','power_source','battery_state','thermal'))


def prepare():
    path=OUT/'plan.json'
    if path.exists(): raise ValueError('PLAN_EXISTS_NO_OVERWRITE')
    if base.command(['git','status','--porcelain']).stdout: raise ValueError('COMMIT_BEFORE_PLAN')
    artifact=base.artifacts()
    plan={'classification':'E5B_LATENCY','source_sha':base.command(['git','rev-parse','HEAD']).stdout.strip(),
          'source_inputs':base.sources(),'artifact':artifact,'trials':schedule(),
          'screen':'foreground; operator confirms brightness unchanged; numeric brightness NOT_COLLECTED',
          'gui_method':'cua_repl native accessibility clicks; fixed A/B and pause25 only',
          'replacement_policy':'NONE; failed/excluded attempts remain in denominator'}
    OUT.mkdir(parents=True,exist_ok=True);base.save(path,plan)
    print(json.dumps({'status':'PREPARED','attempts':len(plan['trials']),'warmups':12,'measurements':120,'overhead':12}))


def verify(plan):
    if base.command(['git','rev-parse','HEAD']).stdout.strip()!=plan['source_sha'] or base.command(['git','status','--porcelain']).stdout:
        raise ValueError('SOURCE_CHANGED')
    if base.sources()!=plan['source_inputs'] or base.artifacts()['artifact_hash']!=plan['artifact']['artifact_hash']:
        raise ValueError('ARTIFACT_CHANGED')


def collect(item, plan):
    conflict=competing_work()
    if any(conflict.values()): raise ValueError('CONCURRENT_BUILD_OR_APP')
    env=base.environment(); directory=OUT/'trials'/('%03d'%item['index']);directory.mkdir(parents=True,exist_ok=False)
    exe=APP/'Contents/MacOS/aether-desktop'; childexe=str(APP/'Contents/Resources/sidecar/node')
    args=[str(exe)]+(['--benchmark-smoke'] if item['instrumented'] else [])
    launch=time.monotonic_ns()
    proc=subprocess.Popen(args,cwd=directory,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,
        env={'PATH':'/usr/bin:/bin','AETHER_BENCH_TRIAL':item['trial_id'],'AETHER_BENCH_VARIANT':item['variant']})
    owned=base.identity(proc.pid)
    if not owned or owned[2]!=str(exe):
        proc.terminate();proc.wait(timeout=5);raise ValueError('HOST_IDENTITY')
    q=queue.Queue(256);overflow=threading.Event()
    def read():
        while True:
            line=proc.stderr.readline(4097)
            if not line:return
            if len(line)>4096:overflow.set();continue
            try:q.put_nowait((time.monotonic_ns(),line.decode('utf-8')))
            except (queue.Full,UnicodeDecodeError):overflow.set()
    thread=threading.Thread(target=read,daemon=True);thread.start()
    events=[];children={};external={};instance=None;problem=None;metrics={};last_check=0
    print(json.dumps({'index':item['index'],'variant':item['variant'],'fixture':item['fixture'],'instrumented':item['instrumented'],'pid':proc.pid}),flush=True)
    base.save(OUT/'current.json',dict(item,pid=proc.pid,status='RUNNING'))
    try:
        while proc.poll() is None or not q.empty() or thread.is_alive():
            if time.monotonic_ns()-launch>90_000_000_000:raise ValueError('TIMEOUT')
            if overflow.is_set():raise ValueError('OUTPUT_LIMIT')
            if time.monotonic()-last_check>1:
                last_check=time.monotonic()
                if any(competing_work().values()):problem='ENVIRONMENT_CONFLICT'
            try:received,line=q.get(timeout=.02)
            except queue.Empty:continue
            if line.startswith(base.PREFIX):
                e=base.validate_event(json.loads(line[len(base.PREFIX):]))
                if e['process_pid']!=proc.pid or e['trial_id']!=item['trial_id']:raise ValueError('EVENT_IDENTITY')
                events.append(e)
                if len(events)>128:raise ValueError('EVENT_LIMIT')
                if e['event_type']=='session_ready' and 'session' not in external:external['session']=received
            else:
                match=re.fullmatch(r'AETHER_INSTANCE instance=([a-f0-9]{32}) pid=(\d+) time_ms=\d+ executable=Contents/MacOS/aether-desktop\n',line)
                if match and int(match[2])==proc.pid:instance=match[1]
                match=re.search(r'AETHER_SIDECAR pid=(\d+) parent=(\d+) .*executable=Contents/Resources/sidecar/node ',line)
                if match and int(match[2])==proc.pid:
                    pid=int(match[1]); ident=base.identity(pid)
                    if ident and ident[0]==proc.pid and ident[2]==childexe:children[pid]=ident
                # Common external observation in ON and OFF: same existing fixed diagnostic lines.
                match=re.search(r'^AETHER_FAKE variant=(webview-a|node-b) .* seq=(\d+) type=(start|progress|terminal) value=(|25|50|75|FAKE_OK) state=(running|succeeded) terminal_count=([01]) ',line)
                if match and match[1]==item['variant']:
                    if match[3]=='start':external.setdefault('start',received)
                    if match[3]=='terminal':external.setdefault('terminal',received)
        if proc.returncode!=0:raise ValueError('HOST_EXIT')
        if item['instrumented']:
            accepted,final,frames=base.check_trace(events,item['trial_id'],item['variant'],item['fixture'],proc.pid)
            if not instance or instance!=events[0]['app_instance'] or 'session' not in external:raise ValueError('SESSION_IDENTITY')
            metrics['process_spawn_to_session_ready_observed']=(external['session']-launch)/1e6
            metrics['accept_to_first_progress']=(next(e for e in frames if e['progress']==25)['monotonic_ns']-accepted['monotonic_ns'])/1e6
            metrics['accept_to_terminal']=(final['monotonic_ns']-accepted['monotonic_ns'])/1e6
            if item['fixture']=='pause25':
                cancel=next(e for e in events if e['event_type']=='cancel_confirmed')
                metrics['host_cancel_processing']=cancel['operation_ns']/1e6
                if item['variant']=='node-b':metrics['cancel_confirmed_to_reap']=(next(e for e in events if e['event_type']=='child_reaped')['monotonic_ns']-cancel['monotonic_ns'])/1e6
            if item['variant']=='node-b':
                metrics['node_spawn_to_ready']=(next(e for e in events if e['event_type']=='child_ready')['monotonic_ns']-next(e for e in events if e['event_type']=='child_spawn')['monotonic_ns'])/1e6
        elif events:raise ValueError('OFF_EMITTED_MEASUREMENT')
        if item['fixture']=='normal':
            if not {'start','terminal'}<=external.keys():raise ValueError('MISSING_COMMON_EXTERNAL_MARKER')
            metrics['external_start_to_terminal_observed']=(external['terminal']-external['start'])/1e6
        for value in metrics.values():stats([value])
    except (ValueError,json.JSONDecodeError) as exc:
        problem=str(exc) if re.fullmatch('[A-Z_]+',str(exc)) else 'INVALID_DIAGNOSTIC'
    finally:
        if proc.poll() is None:
            if base.identity(proc.pid)==owned:
                proc.terminate()
                try:proc.wait(timeout=5)
                except subprocess.TimeoutExpired:proc.kill();proc.wait(timeout=5)
            else:problem='HOST_IDENTITY_LOST'
        thread.join(timeout=2)
        deadline=time.monotonic()+5
        while any(base.identity(pid)==ident for pid,ident in children.items()) and time.monotonic()<deadline:time.sleep(.1)
        for pid,ident in children.items():
            if base.identity(pid)==ident:problem='CHILD_REMAINED';os.kill(pid,9)
        proc.stderr.close()
    endenv=base.environment()
    if any(e['load_average'][0] > e['logical_cpu_count'] for e in (env,endenv)):
        problem=problem or 'ENVIRONMENT_HIGH_LOAD'
    if environment_key(env)!=environment_key(endenv):problem=problem or 'ENVIRONMENT_CHANGED'
    result=dict(item,classification='E5B_LATENCY',source_sha=plan['source_sha'],artifact_hash=plan['artifact']['artifact_hash'],
        app_instance=instance,host_pid=proc.pid,host_exit_code=proc.returncode,environment_before=env,environment_after=endenv,
        status='TIMEOUT' if problem=='TIMEOUT' else 'EXCLUDED' if problem and problem.startswith('ENVIRONMENT') else 'FAIL' if problem else 'PASS',
        failure_reason=problem,metrics=metrics,gui='PENDING_OBSERVATION',events=events)
    base.save(directory/'result.json',result);base.save(OUT/'current.json',dict(item,status=result['status']))
    print(json.dumps({'index':item['index'],'status':result['status'],'instance':instance}),flush=True)
    return result


def analyze(plan):
    results=[]
    for item in plan['trials']:
        p=OUT/'trials'/('%03d'%item['index'])/'result.json'
        if not p.exists():continue
        r=json.loads(p.read_text())
        if any(r[k]!=item[k] for k in item) or r['source_sha']!=plan['source_sha'] or r['artifact_hash']!=plan['artifact']['artifact_hash']:raise ValueError('RESULT_PLAN_MISMATCH')
        receipt=p.parent/'gui.json'
        if receipt.exists():
            gui=json.loads(receipt.read_text())
            if set(gui) != {'instance','state','seq','terminal_count','run_id'} or type(gui['run_id']) is not int or gui['run_id'] != 1 or type(gui['seq']) is not int or type(gui['terminal_count']) is not int:
                raise ValueError('GUI_RECEIPT_SCHEMA')
            if gui['instance']!=r['app_instance'] or gui['state']!=('succeeded' if item['fixture']=='normal' else 'cancelled') or gui['seq']!=(5 if item['fixture']=='normal' else 3) or gui['terminal_count']!=1:raise ValueError('GUI_RECEIPT_MISMATCH')
            r['gui']='OBSERVED'
        if r['instrumented'] and r['status']=='PASS':base.check_trace(r['events'],r['trial_id'],r['variant'],r['fixture'],r['host_pid'])
        results.append(r)
    groups=[]
    for phase in ('warmup','measurement','overhead'):
        for variant in ('webview-a','node-b'):
            for fixture in ('normal','pause25'):
                for enabled in (True,False):
                    expected=[x for x in plan['trials'] if (x['phase'],x['variant'],x['fixture'],x['instrumented'])==(phase,variant,fixture,enabled)]
                    if not expected:continue
                    rows=[r for r in results if (r['phase'],r['variant'],r['fixture'],r['instrumented'])==(phase,variant,fixture,enabled)]
                    keys=sorted(set(environment_key(r['environment_before']) for r in rows))
                    for key in keys or [None]:
                        cohort=[r for r in rows if environment_key(r['environment_before'])==key]
                        valid=[r for r in cohort if r['status']=='PASS' and r['gui']=='OBSERVED']
                        names=sorted({k for r in valid for k in r['metrics']})
                        groups.append(dict(phase=phase,variant=variant,fixture=fixture,instrumented=enabled,environment_cohort=key,
                            planned=len(expected),attempted=len(cohort),success=len(valid),failures=sum(r['status']=='FAIL' for r in cohort),
                            timeouts=sum(r['status']=='TIMEOUT' for r in cohort),excluded=sum(r['status']=='EXCLUDED' for r in cohort),
                            gui_pending=sum(r['gui']!='OBSERVED' for r in cohort),metrics={name:stats([r['metrics'][name] for r in valid if name in r['metrics']]) for name in names}))
    complete=all(any(g['phase']=='measurement' and g['variant']==v and g['fixture']==f and g['success']>=30 for g in groups) for v in ('webview-a','node-b') for f in ('normal','pause25'))
    report=dict(classification='E5B_LATENCY',source_sha=plan['source_sha'],artifact_hash=plan['artifact']['artifact_hash'],planned=len(plan['trials']),attempted=len(results),measurement_complete=complete,groups=groups)
    base.save(OUT/'summary.json',report);print(json.dumps(report))


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('action',choices=['prepare','check','run-block','analyze'])
    p.add_argument('--quiet-window-confirmed',action='store_true')
    args=p.parse_args()
    if args.action=='prepare':return prepare()
    plan=json.loads((OUT/'plan.json').read_text())
    if args.action=='analyze':return analyze(plan)
    verify(plan)
    if args.action=='check':print(json.dumps({'status':'PASS','source_sha':plan['source_sha'],'artifact_hash':plan['artifact']['artifact_hash'],'conflict':competing_work()}));return
    if not args.quiet_window_confirmed:raise ValueError('USER_QUIET_WINDOW_REQUIRED')
    pending=[x for x in plan['trials'] if not (OUT/'trials'/('%03d'%x['index'])).exists()]
    for item in pending[:4]:
        verify(plan)
        result=collect(item,plan)
        if result['status']!='PASS':break

if __name__=='__main__':
    try:main()
    except (ValueError,OSError,KeyError,subprocess.SubprocessError) as exc:
        print(json.dumps({'status':'BLOCKED','reason':str(exc) if re.fullmatch('[A-Z_]+',str(exc)) else type(exc).__name__}));raise SystemExit(2)
