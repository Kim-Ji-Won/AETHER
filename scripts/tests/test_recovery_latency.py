import importlib.util
from pathlib import Path
import sys
import unittest
import tempfile
import json
import io
import contextlib
import subprocess
from unittest.mock import patch
from test_benchmark import trace
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import recovery_latency as b
import recovery_environment as env

class RepeatedTests(unittest.TestCase):
    def test_real_process_lock_blocks_second_collector_and_releases(self):
        with tempfile.TemporaryDirectory() as temp, patch.object(b,'OUT',Path(temp)), patch.object(b,'_collect',return_value='released') as collect:
            code="import fcntl,sys; f=open(sys.argv[1],'a'); fcntl.flock(f,fcntl.LOCK_EX); print('ready',flush=True); sys.stdin.read(1)"
            proc=subprocess.Popen([sys.executable,'-c',code,str(b.OUT/'.collector.lock')],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True)
            try:
                self.assertEqual(proc.stdout.readline().strip(),'ready')
                with self.assertRaisesRegex(ValueError,'COLLECTOR_ALREADY_RUNNING'):b.collect({}, {})
                collect.assert_not_called()
            finally:
                proc.communicate('x',timeout=5)
            self.assertEqual(proc.returncode,0)
            self.assertEqual(b.collect({}, {}),'released')

    def test_fixed_schedule_no_replacement_and_excluded_warmup(self):
        plan=b.schedule();self.assertEqual(len(plan),144)
        self.assertEqual(len({p['trial_id'] for p in plan}),144)
        for v in ('webview-a','node-b'):
            for f in ('normal','pause25'):
                for phase,n in [('warmup',3),('measurement',30)]:
                    self.assertEqual(sum(x['variant']==v and x['fixture']==f and x['phase']==phase for x in plan),n)
        measured=[x for x in plan if x['phase']=='measurement' and x['fixture']=='normal']
        self.assertEqual([x['variant'] for x in measured[:4]],['webview-a','node-b','node-b','webview-a'])
    def test_nearest_rank_30_and_invalid_values(self):
        s=b.stats(list(range(30)));self.assertEqual((s['p50'],s['p95'],s['n']),(14,28,30))
        self.assertIsNone(b.stats([])['p95'])
        for v in [float('nan'),float('inf'),-1,True,None]:
            with self.assertRaises(ValueError):b.stats([v])
    def test_environment_changes_split_cohorts(self):
        a=dict(macos_build='25G',architecture='arm64',power_source='AC',battery_state='charging',thermal='NO_WARNING_REPORTED')
        c=dict(a,power_source='BATTERY');self.assertNotEqual(b.environment_key(a),b.environment_key(c))
    def test_summary_keeps_timeout_missing_gui_and_warmups_out(self):
        plan={'source_sha':'c'*40,'artifact':{'artifact_hash':'d'*64},'trials':[]}
        env=dict(macos_build='25G',architecture='arm64',power_source='AC',battery_state='charging',thermal='NO_WARNING_REPORTED')
        old=b.OUT
        try:
            with tempfile.TemporaryDirectory() as temp:
                b.OUT=Path(temp)
                for i,(phase,status,gui) in enumerate([('warmup','PASS',True),('measurement','PASS',True),('measurement','TIMEOUT',True),('measurement','PASS',False)]):
                    item=dict(index=i,trial_id='a'*32,phase=phase,round=i+1,variant='webview-a',fixture='normal',instrumented=True)
                    plan['trials'].append(item)
                    d=b.OUT/'trials'/('%03d'%i);d.mkdir(parents=True)
                    result=dict(item,source_sha='c'*40,artifact_hash='d'*64,app_instance='b'*32,host_pid=100,
                                environment_before=env,status=status,gui='PENDING_OBSERVATION',metrics={'latency':i+1},events=trace())
                    (d/'result.json').write_text(json.dumps(result))
                    if gui:(d/'gui.json').write_text(json.dumps(dict(instance='b'*32,state='succeeded',seq=5,terminal_count=1,run_id=1)))
                with contextlib.redirect_stdout(io.StringIO()):b.analyze(plan)
                result=json.loads((b.OUT/'summary.json').read_text())
                g=next(g for g in result['groups'] if g['phase']=='measurement')
                self.assertEqual((g['attempted'],g['success'],g['timeouts'],g['gui_pending']),(3,1,1,1))
                self.assertEqual(g['metrics']['latency']['n'],1)
                self.assertFalse(result['measurement_complete'])
                receipt=b.OUT/'trials/001/gui.json';r=json.loads(receipt.read_text());r['instance']='e'*32;receipt.write_text(json.dumps(r))
                with self.assertRaises(ValueError):b.analyze(plan)
        finally:b.OUT=old

    def test_off_conditions_are_separate_and_balanced(self):
        plan=[x for x in b.schedule() if x['phase']=='overhead']
        for v in ('webview-a','node-b'):
            for enabled in (True,False):self.assertEqual(sum(x['variant']==v and x['instrumented']==enabled for x in plan),3)


class RecoverySafetyTests(unittest.TestCase):
    def samples(self, gpu=25, window=30):
        return [dict(monotonic_ns=i*5_000_000_000,gpu_percent=gpu,
                     windowserver_cpu_percent=window,ac=True,conflicts={}) for i in range(12)]

    def test_nonzero_gpu_and_exact_threshold_pass(self):
        rows=self.samples(90,100)
        for r in rows[6:]:r.update(gpu_percent=95,windowserver_cpu_percent=110)
        self.assertEqual(env.evaluate(rows)['status'],'PASS')
        rows[-1]['gpu_percent']=100
        self.assertEqual(env.evaluate(rows)['status'],'PASS')  # median, not peak

    def test_drift_power_missing_and_timing_fail_closed(self):
        rows=self.samples()
        for r in rows[6:]:r['gpu_percent']+=5.1
        self.assertEqual(env.evaluate(rows)['status'],'BLOCKED')
        rows=self.samples();rows[0]['ac']=False
        self.assertIn('AC_REQUIRED',env.evaluate(rows)['reasons'])
        rows=self.samples();rows[1]['monotonic_ns']+=2_000_000_000
        self.assertIn('BASELINE_SAMPLE_TIMING',env.evaluate(rows)['reasons'])
        rows=self.samples();rows[1]['gpu_percent']=None
        self.assertEqual(env.evaluate(rows)['status'],'BLOCKED')
        self.assertEqual(env.evaluate(rows[:11])['status'],'BLOCKED')

    def test_helper_vs_node_and_owned_pid(self):
        rows=[(10,1,'/Applications/ChatGPT.app/Contents/Resources/cua_node/bin/node'),
              (11,1,'/Users/test/Library/Application Support/Codex/artifact-sessions/runtime-generations/generation-x/runtime/bin/node'),
              (12,1,'/opt/homebrew/bin/node'),(13,1,'/tmp/AETHER.app/Contents/MacOS/aether-desktop'),
              (14,13,'/tmp/AETHER.app/Contents/Resources/sidecar/node')]
        c=env.classify(rows,owned=(13,14))
        self.assertEqual((c['codex_helper'],c['build_or_server'],c['other_aether']),(2,1,0))
        self.assertEqual(env.classify(rows)['other_aether'],1)

    def test_services_are_allowed_only_during_preparation(self):
        rows=[(1,0,'/opt/homebrew/bin/postgres'),(2,0,'/Applications/Docker.app/Contents/MacOS/com.docker.backend')]
        with patch.object(env,'processes',return_value=rows):
            self.assertFalse(any(env.conflicts(services_stopped=False).values()))
            self.assertTrue(any(env.conflicts().values()))

    def test_plan_seal_rejects_edit(self):
        with tempfile.TemporaryDirectory() as temp,patch.object(b,'OUT',Path(temp)):
            p=b.OUT/'plan.json';p.write_text('{}\n')
            (b.OUT/'plan.sha256').write_text(b.base.sha(p)+'\n')
            b.verify_seal();p.write_text('{"changed":true}\n')
            with self.assertRaisesRegex(ValueError,'PLAN_SEAL_CHANGED'):b.verify_seal()

    def test_baseline_reservation_prevents_retry(self):
        with tempfile.TemporaryDirectory() as temp:
            p=Path(temp)/'baseline.json';p.write_text('{"status":"BLOCKED"}')
            with patch.object(env.time,'sleep') as sleep:
                with self.assertRaises(FileExistsError):env.baseline(p)
                sleep.assert_not_called()
            self.assertEqual(json.loads(p.read_text())['status'],'BLOCKED')

    def test_preserved_trace_contract_all_four_conditions(self):
        for v in ('webview-a','node-b'):
            for f in ('normal','pause25'):
                a,z,frames=b.base.check_trace(trace(v,f),'a'*32,v,f,100)
                self.assertEqual(a['run_id'],z['run_id'])
                self.assertEqual(z['terminal_count'],1)
                self.assertEqual(z['outcome'],'succeeded' if f=='normal' else 'cancelled')

if __name__=='__main__':unittest.main()
