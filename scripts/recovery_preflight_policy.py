"""Issue 44 functional-preflight policy: read-only service checks, advisory drift."""
import json
import time
from pathlib import Path
import recovery_environment as environment
import recovery_process as process
import recovery_latency as recovery


def idle_services(counts):
    result=dict(postgres='NOT_RUNNING',docker='NOT_RUNNING')
    if counts.get('postgres'):
        rows=[r for r in environment.processes() if Path(r[2]).name=='postgres']
        if not rows:raise ValueError('POSTGRES_IDENTITY_UNAVAILABLE')
        identities={}
        for pid,parent,exe in rows:
            ident=process.identity(pid)
            resolved=str(Path(exe).resolve())
            if not ident or ident[0]!=parent or ident[2]!=resolved or not resolved.startswith('/opt/homebrew/Cellar/postgresql@16/'):
                raise ValueError('POSTGRES_IDENTITY_UNVERIFIED')
            identities[pid]=ident
        sql="SELECT json_build_object('expected_directory',current_setting('data_directory')='/opt/homebrew/var/postgresql@16','other_clients',count(*),'active',count(*) FILTER (WHERE state='active'),'transactions',count(*) FILTER (WHERE xact_start IS NOT NULL)) FROM pg_stat_activity WHERE pid<>pg_backend_pid() AND backend_type='client backend'"
        r=recovery.base.command(['/opt/homebrew/opt/postgresql@16/bin/psql','-X','--no-password','-h','/tmp','-d','postgres','-At','-v','ON_ERROR_STOP=1','-c',sql])
        if r.returncode:raise ValueError('POSTGRES_IDLE_UNVERIFIED')
        value=json.loads(r.stdout)
        if not value['expected_directory'] or any(value[k] for k in ('other_clients','active','transactions')):
            raise ValueError('POSTGRES_USER_WORK_ACTIVE')
        if any(process.identity(pid)!=ident for pid,ident in identities.items()):raise ValueError('POSTGRES_IDENTITY_CHANGED')
        result['postgres']='IDLE_VERIFIED'
    if counts.get('docker'):
        context=recovery.base.command(['/usr/local/bin/docker','context','inspect','--format','{{json .Endpoints.docker.Host}}'])
        if context.returncode:raise ValueError('DOCKER_CONTEXT_UNAVAILABLE')
        endpoint=json.loads(context.stdout)
        if endpoint!='unix://'+str(Path.home()/'.docker/run/docker.sock'):
            raise ValueError('DOCKER_LOCAL_CONTEXT_UNVERIFIED')
        r=recovery.base.command(['/usr/local/bin/docker','ps','--format','{{.ID}}'])
        if r.returncode:raise ValueError('DOCKER_IDLE_UNVERIFIED')
        if r.stdout.strip():raise ValueError('DOCKER_USER_WORK_ACTIVE')
        result['docker']='IDLE_VERIFIED'
    return result


def safety(counts):
    if counts.get('build_or_server') or counts.get('other_aether'):
        raise ValueError('ENVIRONMENT_CONFLICT')
    return idle_services(counts)


def classify_observation(samples):
    assessed=environment.evaluate(samples)
    reasons=assessed.get('reasons',[])
    drift=[r for r in reasons if r.endswith('_DRIFT')]
    other=[r for r in reasons if r not in drift]
    return dict(status='UNAVAILABLE' if other else ('DRIFT_WARNING' if drift else 'STABLE_OBSERVED'),
                metrics=assessed.get('metrics',{}),reasons=reasons)


def observe(path):
    with path.open('x') as f:json.dump({'status':'IN_PROGRESS'},f)
    samples=[];problem=None
    try:
        time.sleep(60);start=time.monotonic()
        for i in range(12):
            time.sleep(max(0,start+i*5-time.monotonic()))
            env=recovery.base.environment();recovery.ac_gate(env)
            counts=environment.classify(environment.processes());services=safety(counts)
            metrics={}
            for name,read in [('gpu_percent',environment.gpu_percent),('windowserver_cpu_percent',environment.window_percent)]:
                try:metrics[name]=read()
                except (ValueError,OSError):metrics[name]=None
            samples.append(dict(monotonic_ns=time.monotonic_ns(),ac=True,
                                conflicts={k:counts[k] for k in ('build_or_server','other_aether')},services=services,**metrics))
        time.sleep(max(0,start+60-time.monotonic()))
    except (ValueError,OSError) as exc:
        problem=str(exc) if str(exc).replace('_','').isupper() else type(exc).__name__
    result=dict(safety_status='BLOCKED' if problem else 'PASS',failure_reason=problem,
                environment_observation=classify_observation(samples),samples=samples,policy='FUNCTIONAL_PREFLIGHT_V2')
    path.write_text(json.dumps(result,indent=2)+'\n')
    print(json.dumps({k:v for k,v in result.items() if k!='samples'}),flush=True)
    return result
