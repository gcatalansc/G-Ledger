# Confirmación histórica G-Skills, perfil candidato 0.1.0

Autorización adicional de Alfredo «Autorizo, procede», comunicada desde G-Skills Engine III el 2026-09-08 para `PL-GSK-003-confirmation-addendum.md` (G-Skills `a0c9382`). Autoridad adicional separada; specs, perfiles y locks anteriores conservados. Estos contratos preceden a la implementación y no declaran certificación.

## Proceso y mensajes

`project-ledger serve confirmation --stdio --profile gskills-record-confirmation/0.1.0 --database <DB local> --policy <policy.json> --project-ref <project:UUIDv7> --gskills-project-ref <project:mempilot> --observed-at <RFC3339 UTC>`.

Todos los argumentos vienen de configuración local fijada. La DB se abre read-only, sin crear ni migrar. Framing `Content-Length`; JSON-RPC 2.0. Únicamente `ledger.manifest` con params `{}` y `ledger.command_confirmation` con params `{"request": <request.schema.json>}`. No acepta submit, history STATE ni paths del cliente.

`v0.1/` contiene cuatro schemas cerrados draft 2020-12, fixtures sintéticas de contrato, mapping, loss report y lock de hashes de bytes. Las fixtures de respuesta no son evidencia de ejecución nativa. Manifest dinámico y policy están ligados por digest; `schema_sha256` fija los cuatro schemas sin ciclo con el lock. El consumidor fija adicionalmente el lock, ejecutable y manifest observado por hashes propios.

Scope de solicitud NATIVO: `actor_ref` equivale a `RecordSinkPolicy.native_actor.actor_id`; `trust_domain` es `TRUSTED_FOR_RECORDING`, no el dominio externo de G-Skills. Tipo de comando y clave se conservan exactamente. La nueva policy de lectura contiene `native_actor_ref`, `native_trust_domain`, allowlist de los cinco tipos nativos del sink, ambas identidades de proyecto y `read_authorized=true`. Este scope no concede escritura ni sustituye la política local.

`authority_binding_ref=project-ledger://authority/confirmation/{policy.digest}`. `confirmation_ref=project-ledger://confirmations/{request.digest}` y `provenance_ref=project-ledger://provenance/confirmation/{request.digest}`. Los digests incluyen el prefijo `sha256:` dentro de esas refs. El digest de cada objeto se calcula con JSON canónico nativo quitando únicamente su campo `digest`.

El cuerpo del digest del efecto tiene exactamente `project_ref`, `command_ref`, `native_command_digest`, `native_result_digest`, `result_cut_head_digest`, `previous_state_version`, `new_state_version`, `emitted_event_refs`, conservando el orden de la lista. El resultado durable, sus índices y el scope, cadena, cortes y causalidad se comprueban en un único snapshot transaccional. Un replay observado no reemplaza el resultado durable.

## Límites y errores

Request 32768 bytes, response 262144 bytes, 256 refs, timeout del proceso 5000 ms; edad máxima de observación 30 s y tolerancia de reloj 2 s. Los límites de bytes corresponden al contenido JSON-RPC, sin cabecera. Policy y params también están sujetos al límite de entrada. El host mata/recolecta el proceso al vencer el tiempo y verifica frescura contra su reloj. La CLI comprueba UTC y orden/frescura respecto de su `observed_at` fijado. La antigüedad del corte histórico no afecta su validez. Defensa interna de lectura: máximo 100000 eventos y 64 MiB de JSON del snapshot; exceso produce READ_LIMIT_EXCEEDED. No hay VERIFIED truncado.

Solicitud malformada/schema/digest inválido: -32602. Método desconocido: -32601. Framing excesivo/inválido: proceso falla sin prueba. Una solicitud válida obtiene VERIFIED, NOT_FOUND, UNAVAILABLE o CONTRADICTION según los enums de respuesta. Error de identidad/policy/scope no devuelve datos del proyecto solicitado. La indisponibilidad conserva el resultado previamente conocido en el host.

VERIFIED es una comprobación ejecutada por el proveedor fijado localmente, no una firma ni prueba autónoma. `current_cut > result_cut` sólo permite clasificar avance después de validar la prueba completa. No se exportan cuerpos de eventos, episodios, logs ni ProjectState.
