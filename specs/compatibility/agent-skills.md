# Compatibilidad Agent Skills/Codex

- Perfil: `project-ledger-agent-skill/0.2.0`.
- Skill: `project-ledger-continuity` v0.2.0.
- Tool requerido: `project-ledger >=0.2.0,<0.3.0`.
- Plataformas declaradas: Linux, macOS y Windows.
- Contratos: manifest y workflow cerrados, registry/fixture manifest raw-locked, 7 triggers y 5 políticas ejecutadas offline.
- Estado: `state_authority=false`, `core_dependency=false`, `certified_compatibility=false`.
- Start/resume: probe de tool y resume canónico antes de planificar; freshness no-current obliga reconciliación.
- Writes: sólo por command boundary con expected version/idempotency cuando aplica; nunca DB directa ni edición de Markdown generado.
- Completion: la skill puede solicitar evaluación/cierre al motor, pero no verificar gates ni completar fases.
- Degradación: tool/subcommand ausente detiene operaciones Ledger; version conflict exige resume/replan; outcome unknown exige reconcile del intento original.
- Handoff local: init→resume→checkpoint→duplicate replay→resume PASS sobre SQLite real, con resume read-only y cadena intacta.
- No demostrado: obediencia universal de un modelo/agente, enforcement fuera del CLI, sandbox universal, integración remota, certificación externa o readiness de release S14.
