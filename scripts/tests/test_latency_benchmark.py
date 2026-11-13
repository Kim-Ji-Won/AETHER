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
import latency_benchmark as b

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

    def test_existing_same_artifact_is_blocked_only_at_launch(self):
        line=f'123 {b.APP}/Contents/MacOS/aether-desktop'
        with patch.object(b.base,'command',return_value=subprocess.CompletedProcess([],0,line)):
            self.assertEqual(b.competing_work()['other_aether_processes'],0)
            self.assertEqual(b.competing_work(include_own_apps=True)['other_aether_processes'],1)

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

if __name__=='__main__':unittest.main()
