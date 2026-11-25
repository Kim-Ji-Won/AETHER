import json
from pathlib import Path
import sys
import unittest
from types import SimpleNamespace
from unittest.mock import patch
sys.path.insert(0,str(Path(__file__).resolve().parents[1]))
import recovery_preflight_policy as p

class SplitPolicyTests(unittest.TestCase):
    def samples(self):
        return [dict(monotonic_ns=i*5_000_000_000,gpu_percent=30 if i<6 else 21,windowserver_cpu_percent=30,ac=True,conflicts={}) for i in range(12)]
    def reply(self,value):return SimpleNamespace(returncode=0,stdout=value)
    def test_same_drift_is_advisory_for_preflight_and_blocking_for_resource(self):
        samples=self.samples()
        self.assertEqual(p.classify_observation(samples)['status'],'DRIFT_WARNING')
        self.assertEqual(p.environment.evaluate(samples)['status'],'BLOCKED')
    def test_missing_optional_metric_does_not_become_stable(self):
        samples=self.samples();samples[0]['gpu_percent']=None
        self.assertEqual(p.classify_observation(samples)['status'],'UNAVAILABLE')
    def test_external_node_cannot_be_exempted_by_idle_services(self):
        with patch.object(p,'idle_services') as services:
            with self.assertRaisesRegex(ValueError,'ENVIRONMENT_CONFLICT'):
                p.safety(dict(build_or_server=1,other_aether=0,postgres=1,docker=1))
            services.assert_not_called()
    def test_postgres_user_connection_blocks_even_when_idle_transaction(self):
        exe='/opt/homebrew/Cellar/postgresql@16/16.13/bin/postgres'
        reply=self.reply(json.dumps(dict(expected_directory=True,other_clients=1,active=0,transactions=0)))
        with patch.object(p.environment,'processes',return_value=[(7,1,exe)]),patch.object(p.process,'identity',return_value=(1,123,exe)),patch.object(p.recovery.base,'command',return_value=reply):
            with self.assertRaisesRegex(ValueError,'POSTGRES_USER_WORK_ACTIVE'):p.idle_services({'postgres':1})
    def test_docker_local_zero_containers_allowed_and_active_blocked(self):
        endpoint='unix://'+str(Path.home()/'.docker/run/docker.sock')
        for containers,expected in [('',True),('container-id\n',False)]:
            with patch.object(p.recovery.base,'command',side_effect=[self.reply(json.dumps(endpoint)),self.reply(containers)]):
                if expected:self.assertEqual(p.idle_services({'docker':2})['docker'],'IDLE_VERIFIED')
                else:
                    with self.assertRaisesRegex(ValueError,'DOCKER_USER_WORK_ACTIVE'):p.idle_services({'docker':2})
    def test_unknown_remote_docker_never_queried(self):
        with patch.object(p.recovery.base,'command',return_value=self.reply(json.dumps('tcp://example.invalid:2375'))) as command:
            with self.assertRaisesRegex(ValueError,'DOCKER_LOCAL_CONTEXT_UNVERIFIED'):p.idle_services({'docker':1})
            self.assertEqual(command.call_count,1)
    def test_postgres_identity_mismatch_blocks_query(self):
        exe='/opt/homebrew/Cellar/postgresql@16/16.13/bin/postgres'
        with patch.object(p.environment,'processes',return_value=[(7,1,exe)]),patch.object(p.process,'identity',return_value=(2,123,exe)),patch.object(p.recovery.base,'command') as command:
            with self.assertRaisesRegex(ValueError,'POSTGRES_IDENTITY_UNVERIFIED'):p.idle_services({'postgres':1})
            command.assert_not_called()
    def test_postgres_symlink_resolves_to_same_native_identity(self):
        exe='/opt/homebrew/Cellar/postgresql@16/16.13/bin/postgres'
        reply=self.reply(json.dumps(dict(expected_directory=True,other_clients=0,active=0,transactions=0)))
        with patch.object(p.environment,'processes',return_value=[(7,1,'/opt/homebrew/opt/postgresql@16/bin/postgres')]),patch.object(p.Path,'resolve',return_value=Path(exe)),patch.object(p.process,'identity',return_value=(1,123,exe)),patch.object(p.recovery.base,'command',return_value=reply):
            self.assertEqual(p.idle_services({'postgres':1})['postgres'],'IDLE_VERIFIED')

if __name__=='__main__':unittest.main()
