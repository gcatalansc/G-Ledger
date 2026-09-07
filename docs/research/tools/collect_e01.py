"""Verify E01 packets and collect descriptive, source-reviewed results.

No participant answers are generated or modified here. Private raw responses
remain in .tooling, bound by hashes in the shareable result.
"""
import hashlib
import json
import statistics
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
RESEARCH = ROOT / 'docs/research'
STUDY = ROOT / '.tooling/research/e01'


def timestamp(value):
    if value.endswith(' UTC'):
        return datetime.strptime(value, '%Y-%m-%d %H:%M:%S UTC').replace(tzinfo=timezone.utc)
    return datetime.fromisoformat(value.replace('Z', '+00:00'))


def main():
    registration_raw = (RESEARCH/'e01/registration.json').read_bytes()
    assert registration_raw == (STUDY/'registration.frozen.json').read_bytes()
    registration = json.loads(registration_raw)
    reviews_raw = (RESEARCH/'e01/researcher-review.json').read_bytes()
    reviews = json.loads(reviews_raw)['reviews']
    runtime = json.loads((ROOT/'.tooling/technical-remediation/runtime-verification.json').read_text(encoding='utf-8'))
    binary = Path(runtime['runtime_directory'])/'project-ledger.exe'
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == registration['binary_sha256']
    rows, integrity = [], []
    for task in registration['tasks']:
        for condition in ['A','B']:
            identity = task['task_id']+condition
            packet = STUDY/identity
            for name, digest in task['input_hashes'][condition].items():
                assert hashlib.sha256((packet/name).read_bytes()).hexdigest() == digest, (identity,name)
            response_raw = (packet/'RESPONSE.json').read_bytes()
            response = json.loads(response_raw)
            assert all(name in response['answer'] for name in ['state','relevant_change','evidence','uncertainty','next_action'])
            start, end = timestamp(response['started_at_utc']), timestamp(response['completed_at_utc'])
            seconds = (end-start).total_seconds()
            assert 0 <= seconds <= registration['time_limit_seconds']
            scores = reviews[identity]['scores']
            pending = any(value is None for value in scores)
            score_min = sum(value or 0 for value in scores)
            row = {'assigned_id':identity,'reported_id':response['participant_id'],'condition':condition,
                   'task_id':task['task_id'],'situation':task['situation'],
                   'response_sha256':hashlib.sha256(response_raw).hexdigest(),
                   'started_at_utc':start.isoformat(),'completed_at_utc':end.isoformat(),
                   'attempt_seconds_reported':seconds,'quality_scores':scores,
                   'score_interval':[score_min,score_min+2*sum(value is None for value in scores)],
                   'meets_rubric_under_researcher_review':None if pending else all(value==2 for value in scores),
                   'scope_review_required':pending,'context_exposure_declared':bool(response['accidental_exposure']),
                   'reported_ledger_commands':len(response['ledger_commands_executed']),
                   'exact_model_identifier_verified':False,'source_files_reported':len(response['sources_read']),
                   'human_minutes':None,'tokens':None,'cost':None}
            if task['situation']=='U2':
                handoff = (packet/'HANDOFF.md').read_bytes()
                row['handoff_sha256'] = hashlib.sha256(handoff).hexdigest()
                row['handoff_bytes'] = len(handoff)
            if condition=='B':
                for name, digest in task['ledger_initial_hashes'].items():
                    assert hashlib.sha256((packet/name).read_bytes()).hexdigest() == digest
                expected_version = 5 if task['situation']=='U2' else 12 if task['task_id']=='T6' else 3
                result = subprocess.run([str(binary),'verify','--locator',str(packet/'ledger.local.json'),'--json'],capture_output=True,timeout=30)
                (STUDY/f'post-{identity}-verify.stdout').write_bytes(result.stdout)
                (STUDY/f'post-{identity}-verify.stderr').write_bytes(result.stderr)
                assert result.returncode == 0
                verified = json.loads(result.stdout)
                assert verified['integrity']=='ok' and verified['state_version']==expected_version
                integrity.append({'id':identity,'integrity':verified['integrity'],'state_version':verified['state_version'],
                                  'findings':len(verified['findings']),'stdout_sha256':hashlib.sha256(result.stdout).hexdigest()})
            rows.append(row)
    pairs=[]
    for task in registration['tasks']:
        a,b = (next(row for row in rows if row['assigned_id']==task['task_id']+condition) for condition in ['A','B'])
        pairs.append({'task':task['task_id'],'situation':task['situation'],
                      'attempt_seconds_A':a['attempt_seconds_reported'],'attempt_seconds_B':b['attempt_seconds_reported'],
                      'difference_B_minus_A':b['attempt_seconds_reported']-a['attempt_seconds_reported'],
                      'score_A':a['score_interval'],'score_B':b['score_interval']})
    result = {'qualification':'EXPLORATORY_LLM_CALIBRATION_ONLY','collected_at_utc':datetime.now(timezone.utc).isoformat(),
              'registration_sha256':hashlib.sha256(registration_raw).hexdigest(),
              'researcher_review_sha256':hashlib.sha256(reviews_raw).hexdigest(),
              'participants':len(rows),'pairs':pairs,'rows':rows,'ledger_integrity':integrity,
              'inputs_unchanged':True,'common_pair_inputs_byte_identical':True,
              'context_exposure_declared_count':sum(row['context_exposure_declared'] for row in rows),
              'quality_clear_count':sum(row['meets_rubric_under_researcher_review'] is True for row in rows),
              'quality_pending_count':sum(row['scope_review_required'] for row in rows),
              'attempt_seconds_median':{condition:statistics.median(row['attempt_seconds_reported'] for row in rows if row['condition']==condition) for condition in ['A','B']},
              'primary_time_until_validated_correctness_available':False,
              'causal_productivity_effect':None,'net_adoption_benefit':None,
              'decision':'No demonstrated net utility advantage; do not promote P1 or change product based on E01.'}
    destination = RESEARCH/'e01/results.json'
    if destination.exists():
        raise ValueError('Preserve earlier collection; choose a new revision explicitly')
    destination.write_bytes((json.dumps(result,ensure_ascii=False,indent=2)+'\n').encode('utf-8'))
    print(json.dumps({key:result[key] for key in ['participants','pairs','inputs_unchanged','context_exposure_declared_count','quality_clear_count','quality_pending_count','attempt_seconds_median']},ensure_ascii=False,indent=2))


if __name__ == '__main__':
    main()
