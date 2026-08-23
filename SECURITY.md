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
