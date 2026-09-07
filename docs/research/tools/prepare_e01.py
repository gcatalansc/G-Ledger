"""Prepare frozen local A/B packets before independent E01 participants run.

Only disposable study Ledgers are written. B is a minimal agent-operated
configuration (progress + checkpoints), not a reconstruction of historical
human approvals or a complete phase/gate configuration of the source project.
"""
import hashlib
import json
import random
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
STUDY = ROOT / '.tooling/research/e01'
CASES = [
    ('T1', ['R04'], 'U1',
     'Retoma la remediación de G-Ledger. Determina si la corrección local ya forma parte de la release publicada y si está cubierta por su aceptación. Explica el sujeto de la evidencia y la próxima acción válida.',
     'La remediación local está verificada en 53dc909. La release publicada continúa en 22b3eca, v0.2.0-rc.1; la aceptación previa no se transfiere al binario nuevo.'),
    ('T2', ['R06'], 'U1',
     'Retoma F10 de G-Flow. Determina si puede cerrarse y qué debe suceder para continuar. Distingue evidencia ejecutable, inspección posterior y alcance de cada verificación.',
     'F10 sigue bloqueada por ledger.project_summary. Producto 10/19. La bitácora fija como condición una distribución con el método nativo y una nueva validación bilateral; la revisión posterior fue estática.'),
    ('T3', ['R09'], 'U1',
     'Reconstruye dos momentos: el checkpoint revision-005 y el cierre posterior de D1. Indica qué cambió, qué autoriza y si eso demuestra que D2 está implementada. Responde sólo hasta el cierre D1 del extracto.',
     'revision-005 estuvo pendiente en el checkpoint 6040a8e. La aprobación posterior permite cerrar D1 y ejecutar D2. La aprobación por sí sola no demuestra implementación de D2.'),
    ('T4', ['R07'], 'U2',
     'Incorpora el nuevo resultado SQLite de SOURCES.md a un HANDOFF.md para el siguiente responsable. Aclara qué entorno está migrado, qué alcance tiene la evidencia y cómo proceder con la DB real. No ejecutes migraciones.',
     'El nuevo resultado SQLite de SOURCES.md todavía debe incorporarse al registro de cierre de sesión.'),
    ('T5', ['R08'], 'U2',
     'Incorpora el resultado backend de SOURCES.md a un HANDOFF.md. Conserva los intentos fallidos, lo corregido y lo que sigue sin explicación. Indica una próxima acción razonable sin afirmar una causa que la fuente no demuestre.',
     'La corrida final y el diagnóstico de los fallos de SOURCES.md todavía deben incorporarse al registro de cierre de sesión.'),
    ('T6', [], 'U1',
     'Retoma el fixture C01. Indica cuántas incógnitas materiales siguen abiertas, qué sabe realmente el checkpoint, qué información se pierde en los resúmenes y si hay autoridad para comenzar D3. Identifica la siguiente acción válida.',
     'Hay ocho incógnitas materiales abiertas; el checkpoint quedó invalidado. El texto íntegro del avance y las ocho referencias están disponibles en SOURCES.md. No se registró próxima acción canónica. No se ha proporcionado una aprobación de D3.'),
]


def encoded_json(value):
    return (json.dumps(value, ensure_ascii=False, indent=2) + '\n').encode('utf-8')


def main():
    if STUDY.exists():
        raise ValueError('Refusing to overwrite an existing study')
    STUDY.mkdir(parents=True)
    research = ROOT / 'docs/research'
    manifest = json.loads((research/'case-source-manifest.v0.1.json').read_text(encoding='utf-8'))
    runtime = json.loads((ROOT/'.tooling/technical-remediation/runtime-verification.json').read_text(encoding='utf-8'))
    binary = Path(runtime['runtime_directory'])/'project-ledger.exe'
    expected_binary = '3d240fe74730f8942371f9d2cceeab62d001b6929414339bab455cec97a93438'
    assert hashlib.sha256(binary.read_bytes()).hexdigest() == expected_binary
    calls = []

    def cli(argv):
        result = subprocess.run([str(binary), *argv], capture_output=True, timeout=30)
        ordinal = len(calls)+1
        calls.append({'argv': argv, 'exit_code': result.returncode,
                      'stdout_sha256': hashlib.sha256(result.stdout).hexdigest()})
        (STUDY/f'prepare-{ordinal:03d}.stdout').write_bytes(result.stdout)
        (STUDY/f'prepare-{ordinal:03d}.stderr').write_bytes(result.stderr)
        (STUDY/'preparation-calls.local.json').write_bytes(encoded_json(calls))
        if result.returncode != 0:
            raise RuntimeError(f'Preparation CLI failure {ordinal}')
        return result.stdout.decode('utf-8')

    registration = {'study':'E01','kind':'EXPLORATORY_CALIBRATION','registered_at_utc':datetime.now(timezone.utc).isoformat(),
                    'participants_authorized':12,'participants_per_attempt':1,'seed':20260907,
                    'model':'parent-inherited; no override; exact runtime identifier not exposed by spawn API',
                    'session_context':'fork_turns=none; packet-only instruction; shared filesystem, not OS-enforced blindness',
                    'time_limit_seconds':900,'binary_sha256':expected_binary,
                    'source_commit':'53dc909fc34d2ca03f9ca73db3a1af3b54520610',
                    'primary':'Time until correct handoff, exploratory only; no efficacy threshold',
                    'rubric':['state','temporal_and_subject_scope','evidence_and_reason','uncertainty','valid_next_action'],
                    'points_per_dimension':[0,1,2],
                    'critical_errors':['unsupported authority','claiming unexecuted work','evidence attributed to wrong subject'],
                    'conditions':{'A':'Maintained concise state + frozen source excerpts + ordinary file tools',
                                  'B':'Identical A files + current G-Ledger minimal agent recording workflow; U2 must also record progress and checkpoint'},
                    'limits':['One owner ecosystem; six discovery cases, no holdout.',
                              'B progress/checkpoint configuration does not encode historical human decisions as canonical approvals.',
                              'Model token/cost telemetry is unavailable; do not infer it from bytes.',
                              'No native participant-specific memory is prepopulated; fresh sessions have the same supplied context.',
                              'Preparation is scripted and outside participant clocks; total net benefit cannot be inferred.',
                              'Researcher grading is source-based, condition-aware; critical discrepancies need owner review before promotion.'],
                    'tasks':[], 'dispatch_order':[]}
    for task_id, case_ids, situation, question, brief in CASES:
        sources = []
        for case_id in case_ids:
            case = next(c for c in manifest['cases'] if c['id']==case_id)
            source = manifest['sources'][case['source']]
            raw = (ROOT/'.tooling/research/utility-v01/sources'/f"{case['source']}-{source['sha256']}.txt").read_bytes()
            assert hashlib.sha256(raw).hexdigest() == source['sha256']
            excerpt = '\n'.join(raw.decode('utf-8-sig').splitlines()[case['start_line']-1:case['end_line']])
            sources.append(f"## {case_id}: {source['repo']} / {source['path']}\nCommit {source['commit']}; líneas {case['start_line']}–{case['end_line']}.\nSHA-256 del documento: {source['sha256']}\n\n{excerpt}\n")
        if task_id == 'T6':
            calibration = json.loads((research/'results/calibration-C01.json').read_text(encoding='utf-8'))
            resume = (research/'results/resume-input.json').read_text(encoding='utf-8')
            sources = ['## Fixture C01 — sintético\nTexto íntegro del progreso registrado:\n'+calibration['declared_progress']+
                       '\n\nNo hay autorización humana de D3 en el fixture.\n\nJSON derivado del CLI:\n```json\n'+resume+'\n```\n']
        common = {'TASK.md': f'# {task_id}: {situation}\n\n{question}\n\nDevuelve estado, cambio relevante, evidencia, incertidumbre y próxima acción. No ejecutes trabajo de los proyectos originales.\n',
                  'CURRENT_STATE.md': '# Estado de lectura mantenido para este ejercicio\n\n'+brief+'\n\nLas fuentes fijadas y su alcance están en SOURCES.md.\n',
                  'SOURCES.md': '# Evidencia del caso\n\n'+'\n'.join(sources)}
        task_record = {'task_id':task_id,'case_ids':case_ids,'situation':situation,'input_hashes':{}}
        for condition in ['A','B']:
            participant = task_id+condition
            packet = STUDY/participant
            packet.mkdir()
            for name, body in common.items():
                (packet/name).write_bytes(body.encode('utf-8'))
            if condition == 'B':
                if task_id == 'T6':
                    prior = json.loads((ROOT/'.tooling/research/utility-v01/20260907T220012Z-22221d2c/calls.local.json').read_text(encoding='utf-8'))
                    argv = next(c['argv'] for c in prior if c['label']=='initial-status')
                    locator = {'database':argv[argv.index('--database')+1], 'project_ref':argv[argv.index('--project-ref')+1]}
                else:
                    initialized = cli(['init','--title','E01 '+task_id+' — synthetic index','--summary','Study fixture; not authority over source project',
                                       '--timezone','UTC','--data-root',str(packet/'data')])
                    identity = dict(line.split('=',1) for line in initialized.splitlines() if '=' in line)
                    locator = {key:identity[key] for key in ['database','project_ref']}
                    selection = ['--database',locator['database'],'--project-ref',locator['project_ref']]
                    version = json.loads(cli(['status',*selection,'--json']))['state_version']
                    for ordinal, (command, extra) in enumerate([
                        (['record','progress'],['--summary',brief]),
                        (['checkpoint','create'],['--reason','E01 initial index'])],1):
                        response = json.loads(cli([*command,*selection,'--expected-version',str(version),
                                                   '--idempotency-key',f'e01:{task_id}:prepare:{ordinal}',
                                                   '--actor-id','agent:e01-preparation','--observed-at',datetime.now(timezone.utc).isoformat(),*extra,'--json']))
                        version = response['new_state_version']
                (packet/'ledger.local.json').write_bytes(encoded_json(locator))
                selection = ['--database',locator['database'],'--project-ref',locator['project_ref']]
                for label, command in [('status',['status','--json']),('resume',['resume','--format','json']),('codex',['resume','--format','codex'])]:
                    (packet/f'ledger-{label}.txt').write_bytes(cli([*command,*selection]).encode('utf-8'))
                guide = ('# G-Ledger disponible\n\nEste Ledger es un índice sintético de estudio. Sus fases/identidad no sustituyen las del proyecto fuente. '
                         'Las fuentes completas permanecen en SOURCES.md. La configuración usada sólo registra progreso y checkpoints.\n\n'
                         f'Binario: `{binary}`\nLocalizador: `{packet / "ledger.local.json"}`\n\n'
                         'Puedes leer las salidas capturadas ledger-*.txt o consultar por CLI. Resume acotado no sustituye la fuente. '
                         'No leas SQLite directamente ni uses --as-human.\n\n'
                         'Lectura: `status --locator <localizador> --json`; `resume --locator <localizador> --format json`.\n'
                         'Si la tarea es U2, además de HANDOFF.md registra el avance con `record progress --locator <localizador> '
                         '--expected-version <N> --idempotency-key <clave única> --actor-id agent:'+participant+
                         ' --observed-at <UTC ISO8601> --summary <texto> --json`. Después crea un checkpoint con '
                         '`checkpoint create --database <database> --project-ref <project_ref> --expected-version <N actualizado> '
                         '--idempotency-key <otra clave> --actor-id agent:'+participant+' --observed-at <UTC ISO8601> --reason e01-session-end --json`. '
                         'Usa los nuevos state_version del resultado; no inventes decisiones humanas.\n')
                (packet/'LEDGER.md').write_bytes(guide.encode('utf-8'))
            hashes = {name:hashlib.sha256((packet/name).read_bytes()).hexdigest() for name in common}
            task_record['input_hashes'][condition] = hashes
            if condition == 'B':
                task_record['ledger_initial_hashes'] = {p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in packet.glob('ledger-*.txt')}
        assert task_record['input_hashes']['A'] == task_record['input_hashes']['B']
        registration['tasks'].append(task_record)
    order = [t[0]+c for t in CASES for c in ['A','B']]
    random.Random(20260907).shuffle(order)
    registration['dispatch_order'] = order
    registration['preparation_cli_calls'] = len(calls)
    (research/'e01').mkdir(exist_ok=True)
    (research/'e01/registration.json').write_bytes(encoded_json(registration))
    (STUDY/'registration.frozen.json').write_bytes(encoded_json(registration))
    print(json.dumps({'status':'FROZEN','packets':12,'equal_common_inputs':True,'cli_calls':len(calls),'order':order,
                      'registration_sha256':hashlib.sha256(encoded_json(registration)).hexdigest()}))


if __name__ == '__main__':
    main()
