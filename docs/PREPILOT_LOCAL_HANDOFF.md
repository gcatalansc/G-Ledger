# Entrega local de Project Ledger 0.2.0

Estado: **7/8 verificado; GitHub aplazado por el usuario; NOT_READY para piloto real**. Esta entrega permite revisar el paquete y ensayar con datos descartables. No declara CI aprobada, GA ni certificación. La decisión de aplazar GitHub no dispensa ese criterio del cierre 8/8.

## Qué contiene y cómo identificarla

- ZIP Windows x86_64 del commit limpio `f4b34e591b13199d040ecf7d41a071e21bf63d37`, sin cambios de binario o recursos.
- SHA-256 del ZIP: `0f42c5b1d1f5a714ed790b078e2d463fb27bd19c38932a98d03c78c11be8996c`.
- Guía, informes de preparación y evidencia seleccionada en `evidencia/`; `checksums.sha256` permite comprobar las copias. Las fechas de pruebas anteriores son históricas, no ejecuciones de esta entrega.
- Núcleo sin Cargo, Git, Python ni red. PowerShell 7 se utiliza sólo para ejecutar esta guía. GSP sigue siendo opcional y requiere el runtime fijado por separado; no se instala ni descarga aquí. G-Skills y Radar permanecen fuera del piloto.

El hash debe contrastarse con una fuente de confianza: no constituye firma de código. No ejecutar un ZIP de origen desconocido aunque su manifiesto interno sea coherente. El manual incluido en `resources/docs/WINDOWS_PORTABLE.md` describe el uso previsto; el estado efectivo de preparación lo determina el informe externo de esta entrega.

## Ensayo descartable reproducible

Abra PowerShell 7 en la carpeta de entrega que contiene `project-ledger-0.2.0-windows-x86_64.zip`. Copie los cuatro bloques, en orden, en la misma consola. Cada ensayo crea una carpeta nueva en TEMP; no reutiliza ni borra instalaciones o proyectos. No cambie las rutas de datos por las de un proyecto real. Ante cualquier excepción, deténgase y conserve la carpeta para inspección.

### 1. Verificar y extraer en un destino nuevo

```powershell
$ErrorActionPreference = 'Stop'
$ledgerZip = (Resolve-Path -LiteralPath './project-ledger-0.2.0-windows-x86_64.zip').Path
$ledgerExpectedHash = '0f42c5b1d1f5a714ed790b078e2d463fb27bd19c38932a98d03c78c11be8996c'
if ((Get-FileHash -LiteralPath $ledgerZip -Algorithm SHA256).Hash -ne $ledgerExpectedHash) {
    throw 'SHA-256 del ZIP incorrecto: no extraer ni ejecutar'
}
$ledgerTrialRoot = Join-Path ([IO.Path]::GetTempPath()) ('Ledger ensayo local á ' + [guid]::NewGuid().ToString('N'))
if (Test-Path -LiteralPath $ledgerTrialRoot) { throw 'El destino ya existe' }
$null = New-Item -ItemType Directory -Path $ledgerTrialRoot
$ledgerInstall = Join-Path $ledgerTrialRoot 'instalación nueva'
[IO.Compression.ZipFile]::ExtractToDirectory($ledgerZip, $ledgerInstall)
$ledgerBinary = Join-Path $ledgerInstall 'project-ledger.exe'
$ledgerPackage = Get-Content -LiteralPath (Join-Path $ledgerInstall 'package.json') -Raw | ConvertFrom-Json
if ($ledgerPackage.source_commit -ne 'f4b34e591b13199d040ecf7d41a071e21bf63d37' -or $ledgerPackage.source_tree_dirty) {
    throw 'El manifiesto no corresponde al candidato limpio'
}
$ledgerCalls = [Collections.Generic.List[object]]::new()
function Invoke-LedgerGuide([string[]]$Arguments, [int]$ExpectedExit = 0) {
    $start = [Diagnostics.ProcessStartInfo]::new($ledgerBinary)
    $start.WorkingDirectory = $ledgerTrialRoot
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.Environment.Clear()
    foreach ($key in @('SystemRoot', 'TEMP', 'TMP')) {
        $start.Environment[$key] = [Environment]::GetEnvironmentVariable($key)
    }
    $start.ArgumentList.Add('--json')
    foreach ($argument in $Arguments) { $start.ArgumentList.Add($argument) }
    $process = [Diagnostics.Process]::Start($start)
    try {
        $stdout = $process.StandardOutput.ReadToEndAsync()
        $stderr = $process.StandardError.ReadToEndAsync()
        if (-not $process.WaitForExit(60000)) {
            $process.Kill($true)
            $null = $process.WaitForExit(2000)
            throw 'Tiempo agotado en un proceso propio del ensayo'
        }
        if ($process.ExitCode -ne $ExpectedExit) {
            throw "Código inesperado: $($process.ExitCode); $($stderr.Result); $($stdout.Result)"
        }
        $ledgerCalls.Add(@{operation=$Arguments[0]; exit_code=$process.ExitCode})
        return ($stdout.Result | ConvertFrom-Json)
    } finally { $process.Dispose() }
}
foreach ($family in @('resources', 'specs', 'contracts', 'release')) {
    $null = Invoke-LedgerGuide @($family, 'verify')
}
```

Los procesos Ledger de este ensayo sólo reciben SystemRoot/TEMP/TMP, sin PATH, credenciales o herramientas del checkout. La función invoca únicamente este binario local identificado; no es una sandbox para ejecutar código externo hostil.

### 2. Inicializar, recuperar contexto y registrar una nota

```powershell
$ledgerInit = Invoke-LedgerGuide @('init', '--title', 'Ensayo descartable de entrega local', '--data-root', (Join-Path $ledgerTrialRoot 'datos nuevos'))
$ledgerFields = @{}
foreach ($line in ($ledgerInit.output -split '\r?\n')) {
    if ($line -match '^(project_ref|database)=(.+)$') { $ledgerFields[$Matches[1]] = $Matches[2] }
}
if (-not $ledgerFields.database -or -not $ledgerFields.project_ref) { throw 'Init no devolvió la selección completa' }
$ledgerSelection = @('--database', $ledgerFields.database, '--project-ref', $ledgerFields.project_ref)
$ledgerBefore = Invoke-LedgerGuide (@('resume', '--format', 'json') + $ledgerSelection)
if ($ledgerBefore.state_version -ne 1) { throw 'La base no es un proyecto recién creado' }
$ledgerObservedAt = [DateTimeOffset]::UtcNow.ToString('o')
$ledgerNote = Invoke-LedgerGuide (@('record', 'note', '--kind', 'LIMITATION', '--text', 'Ensayo local descartable. No aprueba gates ni fases.', '--actor-id', 'agent:local-rehearsal', '--observed-at', $ledgerObservedAt, '--expected-version', '1', '--idempotency-key', 'local-rehearsal:note') + $ledgerSelection)
if ($ledgerNote.status -ne 'ACCEPTED' -or $ledgerNote.new_state_version -ne 2) { throw 'Nota no aceptada como se esperaba' }
```

La identidad del ensayo es un agente con capacidad de registro, no una aprobación humana. No añade `--as-human`, autoridades, fases ni gates. No copie secretos o información de otros proyectos en la nota.

### 3. Checkpoint, reconciliación y nueva lectura

```powershell
$ledgerCheckpointArgs = @('checkpoint', 'create', '--reason', 'Cierre del ensayo local descartable', '--actor-id', 'agent:local-rehearsal', '--observed-at', $ledgerObservedAt, '--expected-version', '2', '--idempotency-key', 'local-rehearsal:checkpoint')
$ledgerCheckpoint = Invoke-LedgerGuide ($ledgerCheckpointArgs + $ledgerSelection)
if ($ledgerCheckpoint.status -ne 'ACCEPTED' -or $ledgerCheckpoint.new_state_version -ne 3) { throw 'Checkpoint no aceptado' }
$ledgerReconcileArgs = @('reconcile', '--command-type', 'CREATE_CONTINUITY_CHECKPOINT', '--actor-id', 'agent:local-rehearsal', '--trust-domain', 'LOCAL_USER', '--idempotency-key', 'local-rehearsal:checkpoint')
$ledgerReconciled = Invoke-LedgerGuide ($ledgerReconcileArgs + $ledgerSelection)
if ($ledgerReconciled.outcome -ne 'RECONCILED' -or $ledgerReconciled.result.digest -ne $ledgerCheckpoint.digest) { throw 'La reconciliación no recuperó el resultado original' }
$ledgerResume = Invoke-LedgerGuide (@('resume', '--format', 'json') + $ledgerSelection)
if ($ledgerResume.state_version -ne 3 -or $ledgerResume.checkpoint_ref -ne $ledgerCheckpoint.resulting_checkpoint_ref) { throw 'Resume no conserva el checkpoint' }
$ledgerMarkdown = Join-Path $ledgerTrialRoot 'continuidad derivada.md'
$null = Invoke-LedgerGuide (@('render', '--output', $ledgerMarkdown) + $ledgerSelection)
$null = Invoke-LedgerGuide (@('verify', '--artifact', $ledgerMarkdown) + $ledgerSelection)
```

`LOCAL_USER` aquí selecciona el ámbito de reconciliación del comando histórico checkpoint; no concede permisos nuevos. Ante una respuesta incierta, conserve actor, tipo, trust y clave: no invente otra clave. El Markdown es una vista derivada, no se edita como autoridad ni reemplaza `resume`.

### 4. Backup privado, restauración nueva y replay

```powershell
$ledgerBackup = Join-Path $ledgerTrialRoot 'respaldo privado'
$null = Invoke-LedgerGuide (@('backup', '--output', $ledgerBackup) + $ledgerSelection)
$ledgerRestoredDb = Join-Path $ledgerTrialRoot 'restauración nueva.db'
$ledgerRestoreArgs = @('restore', '--file', $ledgerBackup, '--database', $ledgerRestoredDb)
$null = Invoke-LedgerGuide $ledgerRestoreArgs
$ledgerRestoredSelection = @('--database', $ledgerRestoredDb, '--project-ref', $ledgerFields.project_ref)
$ledgerRestored = Invoke-LedgerGuide (@('resume', '--format', 'json') + $ledgerRestoredSelection)
if ($ledgerRestored.state_version -ne 3 -or $ledgerRestored.checkpoint_ref -ne $ledgerResume.checkpoint_ref) { throw 'La restauración no conserva el estado' }
$ledgerRestoredResult = Invoke-LedgerGuide ($ledgerReconcileArgs + $ledgerRestoredSelection)
if ($ledgerRestoredResult.result.digest -ne $ledgerCheckpoint.digest) { throw 'El backup no conservó el resultado original' }
$ledgerReplay = Invoke-LedgerGuide ($ledgerCheckpointArgs + $ledgerRestoredSelection)
if ($ledgerReplay.status -ne 'DUPLICATE_REPLAY') { throw 'El replay no fue reconocido' }
$null = Invoke-LedgerGuide $ledgerRestoreArgs 2
$ledgerAudit = Invoke-LedgerGuide (@('audit') + $ledgerRestoredSelection)
if ($ledgerAudit.state_version -ne 3 -or @($ledgerAudit.findings).Count -ne 0) { throw 'La auditoría detectó una divergencia' }
[ordered]@{
    guide_status='PASS'
    source_commit=$ledgerPackage.source_commit
    zip_sha256=$ledgerExpectedHash
    cli_invocations=$ledgerCalls.Count
    state_version=$ledgerAudit.state_version
    checkpoint_ref=$ledgerResume.checkpoint_ref
    result_digest=$ledgerCheckpoint.digest
    replay_status=$ledgerReplay.status
    restored_result_preserved=$true
    restore_existing_rejected=$true
    audit_findings=@($ledgerAudit.findings).Count
    github='DEFERRED_BY_USER'
    prepilot_readiness='NOT_READY'
    disposable_root=$ledgerTrialRoot
} | ConvertTo-Json -Depth 6
```

Resultado esperado: `guide_status=PASS`, versión 3, `DUPLICATE_REPLAY`, cero findings y 19 invocaciones CLI. El rechazo 2 al restaurar otra vez es intencionado: el destino existe y no debe sobrescribirse. Este ensayo no demuestra por sí solo el funcionamiento de gates, evidencia ejecutada o GSP; sus pruebas separadas están identificadas en los informes. No se lee SQLite directamente.

Conserve la ruta temporal que imprime el resultado mientras revise la evidencia. El backup es una copia privada completa; no lo comparta. El ensayo no borra automáticamente esa carpeta. Un ensayo nuevo empieza desde el bloque 1 y no reutiliza claves sobre un proyecto distinto seleccionado a mano.

## Continuidad en Codex y límites pendientes

La skill está en `resources/integrations/agent-skills/project-ledger-continuity/` de la instalación extraída. No se instala globalmente aquí. Para un futuro proyecto autorizado, el flujo será seleccionar DB/project_ref, `resume`, registrar hechos/evidencia autorizados, evaluar sin atribuir aprobación, checkpoint y reconciliación. Las operaciones y argumentos reales constan en la skill; una denegación no autoriza a añadir `--as-human`.

Antes del primer piloto real siguen pendientes la CI exact-SHA y su artefacto alojado. No se hacen más consultas, reintentos ni pushes hasta que el usuario retome GitHub. Después de verificarlo y cerrar 8/8 se elegirá el proyecto del piloto. No hay otra fase de implementación añadida por esta entrega local.
