# Política de seguridad

## Versiones soportadas

Durante la implementación inicial solo la rama `main` está soportada.

## Reporte

No publique secretos, credenciales ni evidencia sensible en issues, logs o fixtures. Reporte de forma privada al propietario del repositorio con:

- versión o commit afectado;
- superficie y precondiciones;
- impacto observado;
- reproducción mínima sin datos sensibles;
- mitigación conocida, si existe.

## Límites obligatorios

- Red, telemetría y raw terminal output están deshabilitados por defecto.
- Ninguna configuración de repositorio no confiable puede habilitar writes, adapters, autoridad, credenciales o hooks.
- No se usa interpolación de shell por defecto, criptografía propia ni `unsafe` sin ADR aceptado.
- Los adapters futuros serán out-of-process o artifact exchange; no compartirán bases de datos privadas.

## Gate de release

- El modelo de amenazas publicado está en `docs/THREAT_MODEL.md`.
- `project-ledger release verify` valida el inventario fijado por SHA-256, 90 AC, 32 escenarios PL, migración, SBOM, packaging, benchmark, compatibilidad y el corpus declarado de siete superficies.
- `secret_persistence_count == 0` se limita al corpus y patrones soportados que declara `release/privacy/corpus-manifest.json`; no es una prueba universal contra formatos desconocidos.
- Cadena rota, schema mismatch, secreto persistido, autoridad elevada u overclaim de certificación son NO-GO.
- El release candidate conserva waivers no críticos explícitos para evidencia hosted Linux/macOS y peak-RSS; ambos bloquean General Availability.
