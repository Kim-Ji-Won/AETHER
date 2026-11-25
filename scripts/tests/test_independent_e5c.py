import io
import json
import contextlib
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import sys
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import independent_e5c as h
from test_benchmark import trace

class IndependentTests(unittest.TestCase):
    def test_fixed_six_preflight_conditions(self):
        self.assertEqual(len(h.CASES),6)
        self.assertEqual(h.CASES[:4],[('webview-a','normal','ordinary'),('webview-a','pause25','ordinary'),('node-b','normal','ordinary'),('node-b','pause25','ordinary')])
        self.assertEqual([c[2] for c in h.CASES[4:]],['deferred_registration','exit_boundary'])
    def test_seal_is_exclusive_and_tamper_detected(self):
        with tempfile.TemporaryDirectory() as t,patch.object(h,'ROOT',Path(t)):
            run='a'*32;folder=Path(t)/run
            h.write_new(Path(t)/'runs.json',{'preflight':run})
            h.seal(folder,{'run_id':run})
            self.assertEqual(h.load('preflight')[1]['run_id'],run)
            with self.assertRaises(FileExistsError):h.seal(folder,{'run_id':run})
            (folder/'plan.json').write_text('{}')
            with self.assertRaisesRegex(ValueError,'PLAN_SEAL_CHANGED'):h.load('preflight')
    def test_independent_gate_requires_matching_complete_sealed_evidence(self):
        with tempfile.TemporaryDirectory() as t:
            folder=Path(t);plan=dict(run_id='a'*32,source_sha='b'*40,artifact_hash='c'*64)
            resource=dict(plan,preflight_run=plan['run_id'])
            evidence=folder/'result.json';evidence.write_text('{}')
            h.write_new(folder/'summary.json',dict(status='PASS',functional_status='PASS',completed=6,evidence={'result.json':h.base.sha(evidence)}))
            with patch.object(h,'load',return_value=(folder,plan)),patch.object(h,'verify'):
                h.preflight_gate(resource)
                with self.assertRaisesRegex(ValueError,'PREFLIGHT_PROVENANCE'):h.preflight_gate(dict(resource,source_sha='d'*40))
                evidence.write_text('{"changed":true}')
                with self.assertRaisesRegex(ValueError,'PREFLIGHT_EVIDENCE_CHANGED'):h.preflight_gate(resource)
                h.stop(folder,'OLD_FAILURE')
                with self.assertRaisesRegex(ValueError,'PREFLIGHT_NOT_PASS'):h.preflight_gate(resource)
    def test_latch_is_not_overwritten(self):
        with tempfile.TemporaryDirectory() as t:
            path=Path(t);h.stop(path,'FIRST');before=(path/'stop.json').read_bytes()
            h.stop(path,'SECOND');self.assertEqual((path/'stop.json').read_bytes(),before)
    def test_gui_receipt_requires_instance_and_real_typed_counters(self):
        gui=dict(instance='a'*32,run_id=1,state='succeeded',seq=5,terminal_count=1,context=1,visible=True)
        h.receipt_valid(gui,'a'*32)
        for bad in (dict(gui,instance='b'*32),dict(gui,run_id=True),dict(gui,visible='yes'),dict(gui,unverified=True)):
            with self.assertRaises(ValueError):h.receipt_valid(bad,'a'*32)
    def test_resource_collector_exit_is_not_session_pass(self):
        with tempfile.TemporaryDirectory() as t:
            folder=Path(t);item=dict(index=0);plan=dict(cases=[item],classification='INDEPENDENT_E5C_MINIMUM_SCOPE',source_sha='a'*40,artifact_hash='b'*64)
            (folder/'00/controls').mkdir(parents=True)
            h.write_new(folder/'00/result.json',dict(item,source_sha=plan['source_sha'],artifact_hash=plan['artifact_hash'],status='COLLECTED',host_exit=0,phases=[]))
            with contextlib.redirect_stdout(io.StringIO()):h.finalize('resource',folder,plan)
            summary=json.loads((folder/'summary.json').read_text())
            self.assertEqual(summary['completed'],0);self.assertEqual(summary['status'],'PARTIAL')
            self.assertTrue((folder/'stop.json').exists())
    def test_resource_duration_missing_samples_and_child_recovery(self):
        plan={'conditions':{'phases_per_session':[{'name':'recovery_after_normal','seconds':15}]}}
        phase=dict(name='recovery_after_normal',start_ns=0,end_ns=15_000_000_000,end_gui={'visible':True})
        rows=[dict(time_ns=i*1_000_000_000,host={'rss_bytes':1},children={}) for i in range(16)]
        h.validate_phase(phase,plan,None,rows)
        with self.assertRaisesRegex(ValueError,'PHASE_TOO_SHORT'):h.validate_phase(dict(phase,end_ns=14_000_000_000),plan,None,rows)
        with self.assertRaisesRegex(ValueError,'HOST_SAMPLE_MISSING'):h.validate_phase(phase,plan,None,rows[:5])
        rows[5]['children']={'2':{'rss_bytes':1}}
        with self.assertRaisesRegex(ValueError,'RECOVERY_CHILD_REMAINED'):h.validate_phase(phase,plan,None,rows)
    def test_reload_failure_requires_real_context_marker_and_progress(self):
        cap=object.__new__(h.Capture);cap.instance='b'*32;cap.item=dict(variant='webview-a',trial_id='a'*32)
        cap.context_lost={1};cap.events=[e for e in trace('webview-a','pause25') if e['event_type']!='cancel_confirmed']
        gui=dict(instance='b'*32,run_id=1,state='failed',seq=3,terminal_count=1,context=2,visible=True)
        cap.trace(gui,'pause25')
        cap.events=[e for e in cap.events if e['event_type']!='event_accepted']
        with self.assertRaisesRegex(ValueError,'RELOAD_TERMINAL'):cap.trace(gui,'pause25')

if __name__=='__main__':unittest.main()
