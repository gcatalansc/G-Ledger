# Compatibilidad Project Radar v0.1

- Source spec SHA-256: `277f356cd96c276b3dddf5ef64197e9531cb3acffdbca94232c23de8b7d68c6b`.
- Perfil Ledger: `project-ledger-radar-artifact/0.1.1`.
- Dirección: Project Ledger → Project Radar.
- Transporte: local artifact exchange.
- Escritura Radar→Ledger: no.
- Contratos locales: 5 schemas, 5 fixtures positivos y 5 negativos fijados por hashes crudos y ejecutados offline.
- Export: proyección de un proyecto o catálogo explícito; JSON canónico, self-digest, no tombstones y cero eventos añadidos.
- Identity: el feed contiene sólo anclas Ledger. Un apply de referencia exige `RadarProjectLink` confirmado con método fuerte; nombre/alias sólo produce candidato.
- Merge: duplicate replay, stale ignored, overlay update, source divergence, project omission y field omission permanecen decisiones distintas y no destructivas.
- Ownership: priority, tags, notes, ideas, chats, tasks, artifacts y presentation se conservan como Radar-owned; Ledger no los exporta ni limpia.
- Status/progress: sólo la tabla normativa; porcentaje unavailable con razón explícita cuando el denominador no es estable.
- Provenance: la cadena conserva Ledger como autoridad de lifecycle incluso cuando Radar la proyecte transitivamente.
- CLI local: `integration radar verify-contracts|export|validate-export` y `catalog radar-export --all` PASS sobre SQLite real, read-only y no-clobber.
- Counterpart extension: requerida y todavía `PENDING`.
- Compatibilidad local del lock: `COMPLETE`; compatibilidad certificada: no.
- No demostrado: código/IndexedDB de Radar, migración/backup counterpart, E2E bilateral, runtime productivo o corrección semántica/científica.
