import copy
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('benchmark', Path(__file__).resolve().parents[1]/'benchmark.py')
b = importlib.util.module_from_spec(spec); spec.loader.exec_module(b)


def event(kind='host_accept', **changes):
    e = dict(schema_version=1,trial_id='a'*32,variant='webview-a',component='host',app_instance='b'*32,
             process_pid=100,run_id=1,request_id=2,generation=3,seq=0,event_type=kind,monotonic_ns=0,
             operation_ns=10,outcome='created',exit_code=None,child_pid=None,progress=0,terminal_count=0,
             result='',fixture='normal')
    e.update(changes); return e


def trace(variant='webview-a', fixture='normal'):
    result = [event(variant=variant, fixture=fixture)]
    if variant == 'node-b':
        result += [event(k,variant=variant,fixture=fixture,child_pid=101) for k in ('child_spawn','child_ready')]
    for seq,progress in enumerate([0,25,50,75,75] if fixture=='normal' else [0,25],1):
        final = seq==5
        result.append(event('event_accepted',variant=variant,fixture=fixture,seq=seq,monotonic_ns=seq*100,
                            progress=progress,terminal_count=int(final),outcome='succeeded' if final else 'running',
                            result='FAKE_OK' if final else ''))
    if fixture=='pause25':
        result.append(event('cancel_confirmed',variant=variant,fixture=fixture,seq=3 if variant=='webview-a' else 2,
                            monotonic_ns=600,progress=25,terminal_count=int(variant=='webview-a'),
                            outcome='cancelled' if variant=='webview-a' else 'cancel_requested'))
    if variant=='node-b':
        result.append(event('child_reaped',variant=variant,fixture=fixture,child_pid=101,exit_code=0 if fixture=='normal' else None,monotonic_ns=700))
        result.append(event('run_final',variant=variant,fixture=fixture,child_pid=101,exit_code=0 if fixture=='normal' else None,
                            monotonic_ns=800,seq=5 if fixture=='normal' else 3,progress=75 if fixture=='normal' else 25,
                            terminal_count=1,outcome='succeeded' if fixture=='normal' else 'cancelled',result='FAKE_OK' if fixture=='normal' else ''))
    return result


def row(value=1, **kw):
    r=dict(trial_id='a'*32,variant='webview-a',source_sha='c'*40,artifact_hash='d'*64,fixture='normal',
           environment_id='e'*32,measurement='latency',value=value,unit='ms',status='PASS',failure_reason=None)
    r.update(kw); return r


class BenchmarkTests(unittest.TestCase):
    def test_real_contract_normal_and_cancel_both_variants(self):
        for variant in b.VARIANTS:
            for fixture in ('normal','pause25'):
                a,z,frames=b.check_trace(trace(variant,fixture),'a'*32,variant,fixture,100)
                self.assertEqual((a['run_id'],z['run_id'],z['terminal_count']),(1,1,1))
                self.assertEqual(len(frames),5 if fixture=='normal' else 2)

    def test_replay_wrong_identity_clock_terminal_and_reap_rejected(self):
        mutations=[]
        for key,val in [('run_id',99),('request_id',99),('generation',99),('trial_id','c'*32),('app_instance','c'*32),('process_pid',101),('seq',9),('monotonic_ns',-1),('progress',50)]:
            t=trace(); t[2][key]=val; mutations.append(t)
        t=trace();t.append(t[-1]);mutations.append(t)
        t=trace();t[-1]['terminal_count']=0;mutations.append(t)
        t=trace();t[3]['monotonic_ns']=1;mutations.append(t)
        for t in mutations:
            with self.assertRaises(ValueError):b.check_trace(t,'a'*32,'webview-a','normal',100)
        t=trace('node-b');t=[e for e in t if e['event_type']!='child_reaped']
        with self.assertRaises(ValueError):b.check_trace(t,'a'*32,'node-b','normal',100)

    def test_closed_schema_missing_nonfinite_boolean_and_private_data(self):
        for value in [float('nan'),float('inf'),-1,True,'1',None]:
            with self.assertRaises(ValueError): b.validate_result(row(value))
        for key in b.FIELDS:
            e=event();del e[key]
            with self.assertRaises(ValueError):b.validate_event(e)
        for key,value in [('extra','private'),('outcome','/private/path'),('operation_ns',float('nan')),('seq',True),('trial_id','arbitrary')]:
            e=event();e[key]=value
            with self.assertRaises(ValueError):b.validate_event(e)

    def test_nearest_rank_counts_units_and_missing_are_not_zero(self):
        data=[row(v) for v in [20,1,7,3]]
        data += [row(None,status='FAIL',failure_reason='INVALID'),row(None,status='TIMEOUT',failure_reason='TIMEOUT'),row(None,status='UNAVAILABLE',failure_reason='NO_SAMPLE')]
        s=b.summarize(data)[0]
        self.assertEqual((s['n'],s['p50'],s['p95'],s['min'],s['max'],s['failures'],s['timeouts'],s['uncollected']),(4,3,20,1,20,1,1,1))
        s=b.summarize([row(None,status='NOT_RUN',failure_reason='NO_SAMPLE')])[0]
        self.assertIsNone(s['p50']);self.assertIsNone(s['min'])
        with self.assertRaises(ValueError):b.summarize([row(0,status='FAIL',failure_reason='INVALID')])

    def test_cpu_cumulative_format_and_one_core_definition(self):
        self.assertEqual(b.cpu_seconds('1:02.50'),62.5)
        self.assertEqual(b.cpu_seconds('01:02:03'),3723)
        self.assertEqual(b.cpu_seconds('1-00:00:01'),86401)
        with self.assertRaises(ValueError):b.cpu_seconds('unknown')


if __name__=='__main__':unittest.main()
