# Inicio rápido de G-Ledger en Windows

Este recorrido instala y comprueba G-Ledger, y crea un proyecto descartable. No modifica un repositorio existente.

## 1. Verificar la descarga

Descargue desde el prerelease oficial:

- `project-ledger-0.2.0-windows-x86_64.zip`;
- `project-ledger-0.2.0-windows-x86_64.zip.sha256`.

En PowerShell 7:

```powershell
$zip = '.\project-ledger-0.2.0-windows-x86_64.zip'
$expected = (Get-Content '.\project-ledger-0.2.0-windows-x86_64.zip.sha256').Split(' ')[0].Trim().ToLowerInvariant()
$actual = (Get-FileHash -LiteralPath $zip -Algorithm SHA256).Hash.ToLowerInvariant()
if ($actual -ne $expected) { throw 'El SHA-256 del ZIP no coincide' }
```

El checksum detecta corrupción. No sustituye una firma de código ni protege si un atacante reemplaza simultáneamente el ZIP y el checksum publicado.

## 2. Extraer en una carpeta nueva

```powershell
$install = Join-Path $PWD 'G-Ledger 0.2.0'
if (Test-Path -LiteralPath $install) { throw 'Use una carpeta nueva' }
Expand-Archive -LiteralPath $zip -DestinationPath $install
Set-Location -LiteralPath $install
```

No separe `project-ledger.exe` de `resources/`.

## 3. Verificar la instalación

```powershell
$exe = Join-Path $PWD 'project-ledger.exe'
& $exe --version
& $exe resources verify --json
& $exe specs verify --json
& $exe contracts verify --json
& $exe release verify --json
```

Todos deben terminar con exit code `0`. `release verify` mantiene `overall_certified_compatibility=false`; eso es un límite esperado, no un error.

## 4. Crear un proyecto descartable

```powershell
$dataRoot = Join-Path $PWD 'datos-descartables'
$created = @(& $exe init `
  --title 'Prueba descartable' `
  --summary 'Primer recorrido de G-Ledger' `
  --timezone 'America/Santiago' `
  --data-root $dataRoot)

$projectRef = ($created | Where-Object { $_ -like 'project_ref=*' }) -replace '^project_ref=', ''
$database = ($created | Where-Object { $_ -like 'database=*' }) -replace '^database=', ''
if (-not $projectRef -or -not $database) { throw 'No fue posible leer la identidad creada' }
```

Conserve literalmente `project_ref` y `database`. Para evitar repetirlos, cree un localizador privado:

```powershell
$locator = Join-Path $PWD 'mi-ledger.local.json'
[ordered]@{ database = $database; project_ref = $projectRef } |
  ConvertTo-Json |
  Set-Content -LiteralPath $locator -Encoding utf8NoBOM
```

No publique el localizador si contiene una ruta privada.

## 5. Consultar continuidad

```powershell
& $exe status --locator $locator --json
& $exe resume --locator $locator --format codex
& $exe doctor --locator $locator --json
& $exe verify --locator $locator --json
& $exe audit --locator $locator --json
```

Estas lecturas verifican y reconstruyen; no agregan eventos.

Siguiente lectura: [Tutorial guiado](TUTORIAL.md). Para instalación, rollback y dependencias opcionales, consulte [Windows portable](../WINDOWS_PORTABLE.md).

