# Tutorial: una sesión completa con G-Ledger

Este tutorial parte de la instalación verificada del [inicio rápido](QUICKSTART.md). Muestra el ciclo mínimo que materializa la filosofía de G-Ledger: leer contexto, registrar sólo un avance material, crear un checkpoint y entregar continuidad verificable.

## Preparación

```powershell
$exe = (Resolve-Path '.\project-ledger.exe').Path
$dataRoot = Join-Path $PWD 'tutorial-data'
$created = @(& $exe init `
  --title 'Tutorial G-Ledger' `
  --summary 'Aprender continuidad gobernada' `
  --timezone 'America/Santiago' `
  --data-root $dataRoot)
$projectRef = ($created | Where-Object { $_ -like 'project_ref=*' }) -replace '^project_ref=', ''
$database = ($created | Where-Object { $_ -like 'database=*' }) -replace '^database=', ''
$locator = Join-Path $PWD 'tutorial-ledger.local.json'
[ordered]@{ database = $database; project_ref = $projectRef } |
  ConvertTo-Json | Set-Content -LiteralPath $locator -Encoding utf8NoBOM
```

El archivo `tutorial-ledger.local.json` es comodidad local, no autoridad ni capability.

## Paso 1. Leer antes de escribir

```powershell
$status = & $exe status --locator $locator --json | ConvertFrom-Json
& $exe resume --locator $locator --format codex
```

Una nueva bitácora está en `INITIALIZING`, con `state_version=1`. Las mutaciones siguientes deben usar esa versión esperada. Si otra escritura avanza el estado, G-Ledger rechaza una versión obsoleta en lugar de sobrescribirla.

## Paso 2. Registrar un avance material

```powershell
$observedAt = [DateTimeOffset]::UtcNow.ToString('o')
& $exe record progress `
  --locator $locator `
  --expected-version $status.state_version `
  --idempotency-key 'tutorial-progress-001' `
  --actor-id 'human:owner' `
  --as-human `
  --observed-at $observedAt `
  --summary 'Instalación verificada y primer Ledger creado' `
  --json
```

La clave de idempotencia identifica la intención. Repetir exactamente el mismo comando recupera el resultado sin duplicar el efecto. Reutilizar la clave con otra intención se rechaza.

No registre actividad trivial. Un avance material cambia lo que el siguiente responsable necesita saber: una capacidad terminada, evidencia relevante, una decisión, un blocker, un riesgo o una limitación.

## Paso 3. Comprobar el estado y crear checkpoint

```powershell
$status = & $exe status --locator $locator --json | ConvertFrom-Json
$checkpointAt = [DateTimeOffset]::UtcNow.ToString('o')
& $exe checkpoint create `
  --database $database `
  --project-ref $projectRef `
  --reason 'session-end' `
  --expected-version $status.state_version `
  --idempotency-key 'tutorial-checkpoint-001' `
  --actor-id 'human:owner' `
  --observed-at $checkpointAt `
  --json
```

El checkpoint captura el estado N y se registra como evento N+1. No se edita ni reemplaza; puede quedar stale si cambia una dependencia material.

## Paso 4. Entregar continuidad

```powershell
& $exe resume --locator $locator --format codex
& $exe resume --locator $locator --format json
```

La vista Codex es compacta. La vista JSON conserva referencias completas. Ambas provienen del stream verificado; no leen una narración manual ni todo el historial para reconstruir el contexto esencial.

## Paso 5. Renderizar una vista humana

```powershell
$view = Join-Path $PWD 'BITACORA_PROYECTO.md'
& $exe render --locator $locator --output $view --privacy minimal-local --json
```

El Markdown es una proyección. Si se modifica, `audit --artifact` detecta la divergencia; para corregirla, regenere desde el stream. Nunca edite la vista para cambiar el estado canónico.

## Paso 6. Cerrar con verificaciones

```powershell
& $exe doctor --locator $locator --json
& $exe verify --locator $locator --json
& $exe audit --locator $locator --artifact $view --json
```

`doctor` diagnostica almacenamiento y recuperación; `verify` comprueba cadena y proyección; `audit` busca findings semánticos y divergencia de artefactos. Un exit code aislado no autoriza gates o fases: el resultado persistido y el motor propietario conservan esa autoridad.

## Backup y export portable

Use backup para recuperación privada completa:

```powershell
& $exe backup --locator $locator --output (Join-Path $PWD 'backup-privado') --json
```

Use export portable para trasladar historia compartible minimizada:

```powershell
& $exe export --locator $locator --format portable --output (Join-Path $PWD 'ledger-portable.json') --json
```

El backup conserva SQLite/WAL, resultados e idempotencia. El portable conserva la historia contractual, pero excluye resultados operacionales privados e idempotencia. No son intercambiables.

Continúe con la [Guía de usuario](../operations/USER_GUIDE.md) y la [Referencia operativa de la CLI](../CLI_OPERATIONS.md).

