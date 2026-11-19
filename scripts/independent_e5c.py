#!/usr/bin/env python3
"""Issue 44: six HARNESS_PREFLIGHT cases, then six independent resource sessions.

No arbitrary executable, PID, repeat count, old-plan override or UI automation API.
GUI receipts come from the operator; native process/ledger evidence is separate.
"""
import argparse
import fcntl
import json
import os
from pathlib import Path
import queue
import re
import subprocess
import threading
import time
import uuid
import recovery_latency as recovery
from recovery_latency import base, process, environment
import recovery_preflight_policy as functional

ROOT=recovery.ROOT/'target/gate-split-44'
PRIOR=recovery.ROOT/'target/preflight-e5c-44'
CASES=[('webview-a','normal','ordinary'),('webview-a','pause25','ordinary'),
       ('node-b','normal','ordinary'),('node-b','pause25','ordinary'),
       ('node-b','pause25','deferred_registration'),('node-b','pause25','exit_boundary')]
PHASES=['idle','foreground_long60','recovery_after_normal','hidden_long60',
        'recovery_after_hidden','pause25_cancel','recovery_after_cancel',
        'pause25_reload','recovery_after_reload','normal_quit_and_reap']


def write_new(path,value):
    with path.open('x') as f:json.dump(value,f,indent=2);f.write('\n')


def seal(folder,plan):
    folder.mkdir(parents=True,exist_ok=False);write_new(folder/'plan.json',plan)
    write_new(folder/'seal.json',{'sha256':base.sha(folder/'plan.json')})


def load(kind):
    ids=json.loads((ROOT/'runs.json').read_text());run=ids[kind]
    if not re.fullmatch('[a-f0-9]{32}',run):raise ValueError('RUN_ID_INVALID')
    folder=ROOT/run;plan=json.loads((folder/'plan.json').read_text())
    if base.sha(folder/'plan.json')!=json.loads((folder/'seal.json').read_text())['sha256']:
        raise ValueError('PLAN_SEAL_CHANGED')
    if plan['run_id']!=run:raise ValueError('RUN_ID_CHANGED')
    return folder,plan


def preserved():
    original=recovery.original_evidence()
    old=recovery.OUT;manifest=json.loads((old/'evidence-manifest.json').read_text())
    if any(base.sha(old/k)!=v for k,v in manifest.items()):raise ValueError('RECOVERY_EVIDENCE_CHANGED')
    previous=json.loads((PRIOR/'evidence-manifest.json').read_text())
    if any(base.sha(PRIOR/k)!=v for k,v in previous.items()):raise ValueError('PRIOR_PREFLIGHT_EVIDENCE_CHANGED')
    return dict(original_digest=recovery.digest(original),recovery_manifest_sha256=base.sha(old/'evidence-manifest.json'),
                prior_preflight_manifest_sha256=base.sha(PRIOR/'evidence-manifest.json'))


def verify(plan):
    if recovery.harness_git('rev-parse','HEAD')!=plan['source_sha'] or recovery.harness_git('status','--porcelain'):
        raise ValueError('SOURCE_CHANGED')
    if preserved()!=plan['preserved']:raise ValueError('PRESERVED_BASELINE_CHANGED')
    if base.artifacts()['artifact_hash']!=plan['artifact_hash']:raise ValueError('ARTIFACT_CHANGED')


def prepare():
    if ROOT.exists():raise ValueError('EXPERIMENT_EXISTS_NO_RETRY')
    if recovery.harness_git('status','--porcelain'):raise ValueError('COMMIT_BEFORE_PLAN')
    art=base.artifacts()
    if art['artifact_hash']!=recovery.EXPECTED_BUNDLE:raise ValueError('WRONG_ARTIFACT')
    provenance=dict(source_sha=recovery.harness_git('rev-parse','HEAD'),artifact_hash=art['artifact_hash'],
                    app_source_sha=recovery.EXPECTED_BUILD,preserved=preserved(),environment_policy=environment.POLICY,
                    protocol='GATE_SPLIT_V2',screen_condition='USER_FIXED_CHATGPT_AND_CODEX_WINDOWS_RETAINED')
    ids={k:uuid.uuid4().hex for k in ('preflight','resource')}
    pre=dict(provenance,classification='HARNESS_PREFLIGHT',run_id=ids['preflight'],
             drift_blocks_execution=False,idle_services_allowed=True,
             cases=[dict(index=i,variant=v,fixture=f,mode=m,trial_id=uuid.uuid4().hex) for i,(v,f,m) in enumerate(CASES)],
             timeout_seconds=180,replacements=False)
    conditions=json.loads((recovery.ROOT/'scripts/recovery-resource-plan.json').read_text())
    conditions['gate']='Same-source sealed HARNESS_PREFLIGHT PASS, clean owned processes, own baseline and latch'
    res=dict(provenance,classification='INDEPENDENT_E5C_MINIMUM_SCOPE',run_id=ids['resource'],
             drift_blocks_execution=True,idle_services_allowed=False,
             preflight_run=ids['preflight'],conditions=conditions,
             cases=[dict(index=n*2+j,variant=v,fixture='resource',mode='resource',trial_id=uuid.uuid4().hex)
                    for n,pair in enumerate(conditions['order']) for j,v in enumerate(pair)],timeout_seconds=400,replacements=False)
    ROOT.mkdir();seal(ROOT/ids['preflight'],pre);seal(ROOT/ids['resource'],res);write_new(ROOT/'runs.json',ids)
    print(json.dumps(dict(status='SEALED',source_sha=provenance['source_sha'],**ids)))


def stop(folder,reason):
    if not (folder/'stop.json').exists():write_new(folder/'stop.json',dict(reason=reason,time_ns=time.monotonic_ns()))


def preflight_gate(resource_plan):
    folder,plan=load('preflight');verify(plan)
    if plan['run_id']!=resource_plan['preflight_run'] or any(plan[k]!=resource_plan[k] for k in ('source_sha','artifact_hash')):
        raise ValueError('PREFLIGHT_PROVENANCE_MISMATCH')
    summary=json.loads((folder/'summary.json').read_text())
    if (folder/'stop.json').exists() or summary.get('functional_status')!='PASS' or summary['completed']!=6:
        raise ValueError('PREFLIGHT_NOT_PASS')
    for name,sha in summary['evidence'].items():
        if base.sha(folder/name)!=sha:raise ValueError('PREFLIGHT_EVIDENCE_CHANGED')


def gate(folder,plan):
    verify(plan)
    if (folder/'stop.json').exists():raise ValueError('STOP_LATCH_SET')
    if plan['classification'].startswith('INDEPENDENT'):preflight_gate(plan)
    preflight=plan['classification']=='HARNESS_PREFLIGHT'
    recovery.gate_observation(services_stopped=not preflight)
    if preflight:functional.safety(environment.classify(environment.processes()))
    collectors_absent()


def collectors_absent():
    # Inspect only Python argv, without persisting or printing unrelated arguments.
    for pid,_,exe in environment.processes():
        if pid==os.getpid() or 'python' not in Path(exe).name.lower():continue
        result=base.command(['/bin/ps','-p',str(pid),'-o','args='])
        if any(name in result.stdout for name in ('recovery_latency.py','recovery_resource.py','independent_e5c.py','benchmark.py')):
            raise ValueError('OTHER_COLLECTOR_PRESENT')


def read_controls(directory,seen):
    values=[]
    for p in sorted((directory/'controls').glob('*.json')):
        if p.name not in seen:
            x=json.loads(p.read_text())
            if set(x)!={'action','phase','receipt'} or x['action'] not in ('begin','end') or x['phase'] not in PHASES:
                raise ValueError('CONTROL_SCHEMA')
            seen.add(p.name);values.append(x)
    return values


def receipt_valid(gui,instance):
    if not isinstance(gui,dict) or set(gui)!={'instance','run_id','state','seq','terminal_count','context','visible'}:
        raise ValueError('GUI_RECEIPT_SCHEMA')
    if gui['instance']!=instance or not re.fullmatch('[a-f0-9]{32}',str(instance)):
        raise ValueError('GUI_INSTANCE_MISMATCH')
    if any(type(gui[k]) is not int or gui[k]<0 for k in ('run_id','seq','terminal_count','context')) or type(gui['visible']) is not bool:
        raise ValueError('GUI_RECEIPT_TYPES')
    if gui['state'] not in ('none','running','succeeded','cancelled','failed'):raise ValueError('GUI_STATE')


def hidden(pid):
    # Read-only LaunchServices process metadata. Hiding/showing is done by native GUI only.
    args=['/usr/bin/lsappinfo','find',f'pid={pid}']
    present=base.command(args)
    yes=base.command(args+['hidden=true']);no=base.command(args+['hidden=false'])
    def found(x):return x.returncode==0 and 'ASN:' in x.stdout
    if not found(present) or found(yes)==found(no):raise ValueError('VISIBILITY_UNAVAILABLE')
    return found(yes)


class Capture:
    def __init__(self,proc,item,ident):
        self.proc=proc;self.item=item;self.ident=ident;self.children={};self.events=[]
        self.instance=None;self.reaped=set();self.spawned=set();self.context_lost=set();self.deferred=[]
        self.q=queue.Queue(512);self.overflow=threading.Event();self.timestamps=[]
        self.thread=threading.Thread(target=self.reader,daemon=True);self.thread.start()
    def reader(self):
        for line in iter(lambda:self.proc.stderr.readline(4097),b''):
            if len(line)>4096:self.overflow.set();continue
            try:self.q.put_nowait((time.monotonic_ns(),line.decode('utf-8')))
            except (queue.Full,UnicodeError):self.overflow.set()
    def drain(self):
        if self.overflow.is_set():raise ValueError('OUTPUT_LIMIT')
        while True:
            try:received,line=self.q.get_nowait()
            except queue.Empty:break
            if line.startswith(base.PREFIX):
                e=json.loads(line[len(base.PREFIX):]);check=dict(e)
                if check.get('fixture')=='long60':check['fixture']='normal'
                base.validate_event(check)
                if e['process_pid']!=self.proc.pid or e['trial_id']!=self.item['trial_id'] or e['variant']!=self.item['variant']:
                    raise ValueError('EVENT_IDENTITY')
                self.events.append(e);self.timestamps.append(received)
                if len(self.events)>128:raise ValueError('EVENT_LIMIT')
                if e['event_type']=='child_spawn':
                    pid=e['child_pid'];self.spawned.add(pid)
                    if self.item['mode']=='deferred_registration':
                        self.deferred.append(dict(pid=pid,received_ns=received,registered_before=pid in self.children))
                    else:
                        child=process.identity(pid)
                        expected=str(recovery.APP/'Contents/Resources/sidecar/node')
                        if child is None:raise ValueError('CHILD_DISAPPEARED_BEFORE_REGISTRATION')
                        if child[0]!=self.proc.pid or child[2]!=expected or process.identity(self.proc.pid)!=self.ident:
                            raise ValueError('STDERR_CHILD_IDENTITY')
                        known=self.children.get(pid)
                        if known is not None and known!=child:raise ValueError('CHILD_IDENTITY_CHANGED')
                        self.children[pid]=child
                if e['event_type']=='child_reaped':self.reaped.add(e['child_pid'])
            m=re.fullmatch(r'AETHER_INSTANCE instance=([a-f0-9]{32}) pid=(\d+) time_ms=\d+ executable=Contents/MacOS/aether-desktop\n',line)
            if m and int(m[2])==self.proc.pid:self.instance=m[1]
            m=re.fullmatch(r'AETHER_CONTEXT_LOST run_id=(\d+) generation=\d+ state=failed terminal_count=1\n',line)
            if m:self.context_lost.add(int(m[1]))
    def audit(self,rows=None):
        counts,audit=process.inspect(self.proc.pid,self.ident,str(recovery.APP/'Contents/Resources/sidecar/node'),self.children,rows)
        if self.item['mode']!='resource':
            services=functional.safety(counts)
            audit.append(dict(kind='IDLE_SERVICES_VERIFIED',services=services))
            counts={k:counts[k] for k in ('build_or_server','other_aether')}
        return counts,audit
    def trace(self,gui,fixture):
        receipt_valid(gui,self.instance)
        events=[dict(e) for e in self.events if e['run_id']==gui['run_id']]
        for e in events:
            if e['fixture']=='long60':e['fixture']='normal'
        if gui['state']=='failed' and gui['run_id'] in self.context_lost and self.item['variant']=='webview-a':
            accepted=[e for e in events if e['event_type']=='host_accept']
            frames=[e for e in events if e['event_type']=='event_accepted']
            if (gui['terminal_count']!=1 or gui['seq']!=3 or len(accepted)!=1
                or accepted[0]['fixture']!='pause25' or [e['seq'] for e in frames]!=[1,2]
                or [e['progress'] for e in frames]!=[0,25]
                or any(e['terminal_count'] for e in frames)):
                raise ValueError('RELOAD_TERMINAL')
            return
        _,end,_=base.check_trace(events,self.item['trial_id'],self.item['variant'],fixture,self.proc.pid)
        if any(gui[k]!=end[j] for k,j in [('state','outcome'),('seq','seq'),('terminal_count','terminal_count')]):
            raise ValueError('GUI_LEDGER_MISMATCH')


def collect(folder,plan,item):
    gate(folder,plan)
    is_preflight=plan['classification']=='HARNESS_PREFLIGHT'
    if is_preflight:
        if json.loads((folder/'environment-observation.json').read_text())['safety_status']!='PASS':raise ValueError('SAFETY_OBSERVATION_NOT_PASS')
    elif json.loads((folder/'baseline.json').read_text())['status']!='PASS':raise ValueError('BASELINE_NOT_PASS')
    directory=folder/f"{item['index']:02d}";directory.mkdir(exist_ok=False);(directory/'controls').mkdir()
    exe=str(recovery.APP/'Contents/MacOS/aether-desktop')
    proc=subprocess.Popen([exe,'--benchmark-smoke'],cwd=directory,stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,
                          env={'PATH':'/usr/bin:/bin','AETHER_BENCH_TRIAL':item['trial_id'],'AETHER_BENCH_VARIANT':item['variant']})
    ident=process.identity(proc.pid)
    if not ident or ident[2]!=exe:
        stop(folder,'HOST_IDENTITY_UNVERIFIED')
        raise ValueError('HOST_IDENTITY_UNVERIFIED')
    cap=Capture(proc,item,ident);began=time.monotonic_ns();next_check=0;samples=[];audits=[];proofs={};problem=None
    held_child=None;held_host=None;phase=None;phase_index=0;phases=[];seen=set();last_status=None
    is_resource=item['mode']=='resource'
    def status(stage):
        nonlocal last_status
        value=dict(index=item['index'],pid=proc.pid,instance=cap.instance,variant=item['variant'],mode=item['mode'],stage=stage,
                   phase=phase['name'] if phase else None,proofs=sorted(proofs))
        if value!=last_status:base.save(folder/'current.json',value);last_status=value
    status('RUNNING')
    try:
        while proc.poll() is None or cap.thread.is_alive() or not cap.q.empty():
            now=time.monotonic_ns()
            if (now-began)/1e9>plan['timeout_seconds']:raise ValueError('TIMEOUT')
            cap.drain()
            sample_tick=False
            if now>=next_check:
                sample_tick=True
                next_check=now+1_000_000_000
                counts,audit=cap.audit();audits+=audit
                if any(counts.values()):raise ValueError('ENVIRONMENT_CONFLICT')
                recovery.ac_gate(base.environment())
                if item['mode']=='deferred_registration' and any(a['kind']=='CHILD_DISCOVERED' for a in audit):
                    discovered={a['pid'] for a in audit if a['kind']=='CHILD_DISCOVERED'}
                    delayed=[d for d in cap.deferred if d['pid'] in discovered and not d['registered_before']]
                    if not delayed:raise ValueError('DEFERRED_REGISTRATION_NOT_OBSERVED')
                    proofs['discovered_before_stderr_registration']=dict(time_ns=now,audit=audit,deferred=delayed,stderr_registration_used=False)
                if is_resource and proc.poll() is None:
                    child_samples={str(p):process.sample(p,i) for p,i in cap.children.items() if process.identity(p)==i}
                    samples.append(dict(time_ns=now,phase=phase['name'] if phase else 'transition',
                                        host=process.sample(proc.pid,ident),children=child_samples))
                if len(audits)>1000 or len(samples)>405:raise ValueError('OBSERVATION_LIMIT')
            if item['mode']=='exit_boundary':
                at25=any(e['event_type']=='event_accepted' and e['progress']==25 for e in cap.events)
                if at25 and held_child is None:
                    held_child=process.capture()
                    counts,audit=cap.audit(held_child)
                    if any(counts.values()) or not cap.children:raise ValueError('BOUNDARY_CAPTURE_FAILED')
                    proofs['child_snapshot_before_exit']=dict(time_ns=now,pids=sorted(cap.children))
                    status('CANCEL_AFTER_SNAPSHOT')
                if held_child and cap.reaped and 'child_absence_reconciled' not in proofs:
                    counts,audit=cap.audit(held_child)
                    gone=[a for a in audit if a['kind']=='CHILD_DISAPPEARED']
                    if any(counts.values()) or not gone:raise ValueError('CHILD_INTERLEAVING_NOT_OBSERVED')
                    proofs['child_absence_reconciled']=dict(time_ns=now,audit=gone)
                    held_host=process.capture();proofs['host_snapshot_before_quit']=dict(time_ns=now)
                    status('QUIT_AFTER_SNAPSHOT')
            if is_resource:
                for control in read_controls(directory,seen):
                    name=control['phase'];gui=control['receipt'];receipt_valid(gui,cap.instance)
                    if name!=PHASES[phase_index]:raise ValueError('PHASE_ORDER')
                    if control['action']=='begin':
                        if phase is not None:raise ValueError('PHASE_ALREADY_ACTIVE')
                        phase=dict(name=name,start_ns=now,begin_gui=gui,event_offset=len(cap.events),hidden_samples=[])
                    else:
                        if phase is None:raise ValueError('PHASE_NOT_ACTIVE')
                        phase['end_ns']=now;phase['end_gui']=gui
                        validate_phase(phase,plan,cap,samples)
                        phases.append(phase);phase=None;phase_index+=1
                # Record actual hide onset; exclude UI setup and restore from this fixed window.
                if phase and phase['name']=='hidden_long60' and sample_tick and proc.poll() is None:
                    start=phase.get('hidden_start_ns')
                    if start is None or now<=start+65_000_000_000:
                        value=hidden(proc.pid)
                        if start is None and value:phase['hidden_start_ns']=now
                        if 'hidden_start_ns' in phase:
                            phase['hidden_samples'].append(dict(time_ns=now,hidden=value))
                status('PHASE_ACTIVE' if phase else 'AWAIT_PHASE')
            elif item['mode']!='exit_boundary':status('RUNNING')
            time.sleep(.02)
        cap.drain()
        if proc.returncode!=0:raise ValueError('HOST_EXIT_NOT_ZERO')
        if item['mode']=='exit_boundary':
            if held_host is None:raise ValueError('HOST_INTERLEAVING_NOT_OBSERVED')
            counts,audit=cap.audit(held_host)
            gone=[a for a in audit if a['kind']=='HOST_DISAPPEARED']
            if any(counts.values()) or not gone:raise ValueError('HOST_INTERLEAVING_NOT_OBSERVED')
            proofs['host_absence_reconciled']=dict(time_ns=time.monotonic_ns(),audit=gone,exit_code=proc.returncode)
        if item['mode']=='deferred_registration' and not proofs:raise ValueError('DISCOVERY_INTERLEAVING_NOT_OBSERVED')
        if is_resource:
            if phase is None or phase['name']!='normal_quit_and_reap':raise ValueError('PHASES_INCOMPLETE')
            phase['end_ns']=time.monotonic_ns();phase['exit_code']=proc.returncode
            if (phase['end_ns']-phase['start_ns'])/1e9>10:raise ValueError('QUIT_DEADLINE')
            phases.append(phase);phase=None
            if len(phases)!=10:raise ValueError('PHASES_INCOMPLETE')
        if cap.spawned!=cap.reaped:raise ValueError('REAP_PROOF_MISSING')
        if any(process.identity(p) is not None for p in cap.children):raise ValueError('CHILD_REMAINED_OR_REUSED')
        if item['variant']=='node-b' and cap.spawned-set(cap.children):raise ValueError('CHILD_START_IDENTITY_MISSING')
        if not is_resource:
            gui=json.loads((directory/'gui.json').read_text());cap.trace(gui,item['fixture'])
    except (ValueError,OSError,subprocess.SubprocessError,json.JSONDecodeError) as exc:
        problem=str(exc) if re.fullmatch('[A-Z_]+',str(exc)) else type(exc).__name__
    finally:
        if proc.poll() is None:
            if process.identity(proc.pid)==ident:
                proc.terminate()
                try:proc.wait(timeout=5)
                except subprocess.TimeoutExpired:problem='HOST_REMAINED'
            else:problem='HOST_IDENTITY_LOST'
        cap.thread.join(timeout=2)
        if not cap.thread.is_alive():proc.stderr.close()
    if problem:stop(folder,problem)
    result=dict(item,classification=plan['classification'],source_sha=plan['source_sha'],artifact_hash=plan['artifact_hash'],
                instance=cap.instance,host_pid=proc.pid,host_start_us=ident[1],host_exit=proc.returncode,
                status='BLOCKED' if problem else 'COLLECTED',failure_reason=problem,events=cap.events,
                event_received_ns=cap.timestamps,context_lost=sorted(cap.context_lost),
                children=[dict(pid=p,ppid=i[0],start_us=i[1]) for p,i in cap.children.items()],
                spawned=sorted(cap.spawned),reaped=sorted(cap.reaped),proofs=proofs,process_audit=audits,phases=phases,samples=samples)
    write_new(directory/'result.json',result);base.save(folder/'current.json',dict(index=item['index'],status=result['status'],failure_reason=problem))
    print(json.dumps(dict(index=item['index'],status=result['status'],failure_reason=problem,proofs=list(proofs))))


def validate_phase(phase,plan,cap,samples):
    name=phase['name'];duration=(phase['end_ns']-phase['start_ns'])/1e9
    spec=next(p for p in plan['conditions']['phases_per_session'] if p['name']==name)
    if 'seconds' in spec and duration<spec['seconds']:raise ValueError('PHASE_TOO_SHORT')
    if 'maximum_seconds' in spec and duration>spec['maximum_seconds']:raise ValueError('PHASE_TIMEOUT')
    rows=[s for s in samples if phase['start_ns']<=s['time_ns']<=phase['end_ns']]
    if 'seconds' in spec:
        if len(rows)<int(duration)-2 or any(s['host'] is None for s in rows):raise ValueError('HOST_SAMPLE_MISSING')
        if any((b['time_ns']-a['time_ns'])/1e9>2 for a,b in zip(rows,rows[1:])):raise ValueError('SAMPLE_GAP')
    gui=phase['end_gui']
    if name in ('foreground_long60','hidden_long60'):
        cap.trace(gui,'normal')
        starts=[(e,t) for e,t in zip(cap.events,cap.timestamps) if e['run_id']==gui['run_id'] and e['event_type']=='host_accept']
        if len(starts)!=1 or starts[0][0]['fixture']!='long60':raise ValueError('LONG_FIXTURE_REQUIRED')
        if gui['run_id']<=phase['begin_gui']['run_id']:raise ValueError('FRESH_RUN_REQUIRED')
        if name=='hidden_long60':
            hidden_rows=phase['hidden_samples']
            if (len(hidden_rows)<60 or not all(s['hidden'] for s in hidden_rows)
                or phase['end_ns']-phase.get('hidden_start_ns',phase['end_ns'])<65_000_000_000):
                raise ValueError('HIDDEN_NOT_PROVEN')
    elif name in ('pause25_cancel','pause25_reload'):
        cap.trace(gui,'pause25')
        if name=='pause25_reload' and gui['context']<=phase['begin_gui']['context']:raise ValueError('RELOAD_NOT_OBSERVED')
    elif name.startswith('recovery'):
        if any(s['children'] for s in rows):raise ValueError('RECOVERY_CHILD_REMAINED')
    if not gui['visible']:raise ValueError('GUI_VISIBILITY_MISMATCH')


def finalize(kind,folder,plan):
    evidence={};complete=0;failures=[]
    for item in plan['cases']:
        d=folder/f"{item['index']:02d}"
        if not (d/'result.json').exists():continue
        r=json.loads((d/'result.json').read_text());evidence[str((d/'result.json').relative_to(folder))]=base.sha(d/'result.json')
        try:
            if any(r[k]!=item[k] for k in item) or any(r[k]!=plan[k] for k in ('source_sha','artifact_hash')):raise ValueError('RESULT_PROVENANCE')
            if r['status']!='COLLECTED' or r['host_exit']!=0:raise ValueError('COLLECTOR_NOT_COMPLETE')
            if kind=='preflight':
                gui=json.loads((d/'gui.json').read_text());receipt_valid(gui,r['instance'])
                _,end,_=base.check_trace(r['events'],item['trial_id'],item['variant'],item['fixture'],r['host_pid'])
                if any(gui[k]!=end[j] for k,j in [('run_id','run_id'),('seq','seq'),('state','outcome'),('terminal_count','terminal_count')]):raise ValueError('GUI_LEDGER_MISMATCH')
                if item['mode']=='deferred_registration' and 'discovered_before_stderr_registration' not in r['proofs']:raise ValueError('INTERLEAVING_MISSING')
                if item['mode']=='exit_boundary' and not {'child_absence_reconciled','host_absence_reconciled'}<=r['proofs'].keys():raise ValueError('INTERLEAVING_MISSING')
                evidence[str((d/'gui.json').relative_to(folder))]=base.sha(d/'gui.json')
            elif [p['name'] for p in r['phases']]!=PHASES:raise ValueError('PHASES_INCOMPLETE')
            for p in sorted((d/'controls').glob('*.json')):evidence[str(p.relative_to(folder))]=base.sha(p)
            complete+=1
        except (ValueError,OSError,KeyError) as exc:failures.append(dict(index=item['index'],reason=type(exc).__name__))
    status='PASS' if complete==6 and not failures and not (folder/'stop.json').exists() else 'PARTIAL'
    if failures:stop(folder,'FINAL_EVIDENCE_INVALID')
    summary=dict(classification=plan['classification'],source_sha=plan['source_sha'],artifact_hash=plan['artifact_hash'],
                 status=status,planned=6,attempted=sum((folder/f"{i:02d}"/'result.json').exists() for i in range(6)),
                 completed=complete,failures=failures,evidence=evidence)
    if kind=='preflight':
        summary['functional_status']=status
        observation=folder/'environment-observation.json'
        summary['environment_observation']=json.loads(observation.read_text())['environment_observation'] if observation.exists() else {'status':'NOT_RUN'}
        if observation.exists():evidence['environment-observation.json']=base.sha(observation)
    write_new(folder/'summary.json',summary);print(json.dumps(summary))


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('action',choices=['prepare','baseline','next','receipt','control','finalize','status'])
    p.add_argument('--kind',choices=['preflight','resource'],default='preflight');p.add_argument('--data')
    args=p.parse_args()
    if args.action=='prepare':return prepare()
    folder,plan=load(args.kind)
    if args.action=='status':print((folder/'current.json').read_text());return
    if args.action in ('receipt','control'):
        x=json.loads(args.data);index=x.pop('index')
        if type(index) is not int or not 0<=index<6:raise ValueError('INDEX_INVALID')
        d=folder/f'{index:02d}'
        if args.action=='receipt':write_new(d/'gui.json',x)
        else:
            files=list((d/'controls').glob('*.json'));write_new(d/'controls'/f'{len(files):03d}.json',x)
        return
    if args.action=='finalize':return finalize(args.kind,folder,plan)
    with (ROOT/'collector.lock').open('a') as lock:
        fcntl.flock(lock,fcntl.LOCK_EX|fcntl.LOCK_NB)
        try:
            gate(folder,plan)
            if args.action=='baseline':
                if args.kind=='preflight':
                    r=functional.observe(folder/'environment-observation.json')
                    if r['safety_status']!='PASS':stop(folder,'SAFETY_OBSERVATION_FAILED')
                else:
                    r=environment.baseline(folder/'baseline.json')
                    if r['status']!='PASS':stop(folder,'BASELINE_FAILED')
                return
            for item in plan['cases']:
                if not (folder/f"{item['index']:02d}").exists():return collect(folder,plan,item)
                if not (folder/f"{item['index']:02d}"/'result.json').exists():raise ValueError('INCOMPLETE_PREVIOUS_CASE')
        except (ValueError,OSError,KeyError,subprocess.SubprocessError) as exc:
            reason=str(exc) if re.fullmatch('[A-Z_]+',str(exc)) else type(exc).__name__
            stop(folder,reason)
            raise

if __name__=='__main__':
    try:main()
    except (ValueError,OSError,KeyError,subprocess.SubprocessError) as exc:
        reason=str(exc) if re.fullmatch('[A-Z_]+',str(exc)) else type(exc).__name__
        print(json.dumps(dict(status='BLOCKED',reason=reason)));raise SystemExit(2)
