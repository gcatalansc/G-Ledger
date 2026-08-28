use super::{
    CliError, Selection,
    portability::{json_out, read_bounded, scan, write_new},
};
use clap::Args;
use project_ledger_application::{
    CommandDraft, CommandProcessor, CoreCommandHandler, FixedClock, prepare_command,
};
use project_ledger_canonical_json::{sha256_bytes, to_canonical_vec};
use project_ledger_domain::{
    Actor, ActorKind, ClaimClassification, ClaimRecord, Command, CommandResultStatus,
    LedgerTimestamp, LegacyImportDecision, RecordClaimPayload, ResourceRef, SchemaVersion,
    StableId, TrustLevel,
};
use project_ledger_legacy_import::{LegacyImporter, LegacyReviewRequest};
use project_ledger_storage_sqlite::SqliteEventStore;
use serde_json::json;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct LegacyArgs {
    #[command(flatten)]
    pub selection: Selection,
    #[arg(long)]
    pub file: PathBuf,
    #[arg(long, conflicts_with = "apply_reviewed")]
    pub preview: bool,
    #[arg(long, conflicts_with = "preview")]
    pub apply_reviewed: bool,
    #[arg(long)]
    pub review: Option<PathBuf>,
    #[arg(long)]
    pub preview_digest: Option<String>,
    #[arg(long)]
    pub review_id: String,
    #[arg(long)]
    pub observed_at: String,
    #[arg(long)]
    pub output: Option<PathBuf>,
    #[arg(long)]
    pub actor_id: Option<String>,
    #[arg(long)]
    pub as_human: bool,
    #[arg(long)]
    pub expected_version: Option<u64>,
    #[arg(long)]
    pub idempotency_key: Option<String>,
}

#[allow(clippy::too_many_lines)] // Keep review, all-command preparation and explicit per-candidate submission in auditable order.
pub fn run(args: &LegacyArgs) -> Result<String, CliError> {
    if !args.preview && !args.apply_reviewed {
        return Err(CliError::input("choose --preview or --apply-reviewed"));
    }
    if args
        .output
        .as_ref()
        .is_some_and(|p| std::fs::symlink_metadata(p).is_ok())
    {
        return Err(CliError::input("review output already exists"));
    }
    let markdown = read_bounded(&args.file, 1024 * 1024)?;
    scan(&markdown)?;
    let at = LedgerTimestamp::parse(&args.observed_at).map_err(CliError::input)?;
    let preview = LegacyImporter::preview(LegacyReviewRequest {
        review_id: StableId::parse(&args.review_id).map_err(CliError::input)?,
        source_uri: ResourceRef::parse(&format!(
            "urn:project-ledger:legacy:{}",
            sha256_bytes(&markdown)
        ))
        .map_err(CliError::input)?,
        // The local absolute path is not exported. Bytes and line ranges bind provenance.
        source_path: "legacy-source.md".to_owned(),
        markdown,
        reviewed_at: at.clone(),
    })
    .map_err(CliError::input)?;
    if args.preview {
        return output(args, &preview);
    }
    if !args.as_human {
        return Err(CliError::new(
            9,
            "POLICY_DENIED",
            "historical import requires explicit local human review",
        ));
    }
    if args.preview_digest.as_deref() != Some(preview.digest.to_string().as_str()) {
        return Err(CliError::new(
            15,
            "LEGACY_SOURCE_DIVERGED",
            "review must bind the exact preview digest",
        ));
    }
    let actor_id = StableId::parse(
        args.actor_id
            .as_deref()
            .ok_or_else(|| CliError::input("--actor-id required"))?,
    )
    .map_err(CliError::input)?;
    let review_file = args
        .review
        .as_ref()
        .ok_or_else(|| CliError::input("--review decisions JSON required"))?;
    let review_bytes = read_bounded(review_file, 1024 * 1024)?;
    scan(&review_bytes)?;
    let decisions: Vec<LegacyImportDecision> = serde_json::from_slice(&review_bytes)
        .map_err(|_| CliError::input("invalid review decisions"))?;
    if decisions.iter().any(|d| d.actor_ref != actor_id) {
        return Err(CliError::new(
            9,
            "POLICY_DENIED",
            "every decision must identify the invoking reviewer",
        ));
    }
    let reviewed =
        LegacyImporter::finalize(&preview, decisions, at.clone()).map_err(CliError::input)?;
    let (database, project) = args.selection.resolve()?;
    args.selection.load()?;
    let initial = args
        .expected_version
        .ok_or_else(|| CliError::input("--expected-version required"))?;
    let key = args
        .idempotency_key
        .as_deref()
        .ok_or_else(|| CliError::input("--idempotency-key required"))?;
    let actor = Actor {
        actor_id: actor_id.clone(),
        actor_kind: ActorKind::Human,
        trust_level: TrustLevel::LocalUser,
        display_name: None,
    };
    let mut commands = Vec::new();
    for (index, proposal) in reviewed.asserted_proposals.iter().enumerate() {
        let candidate = preview
            .candidates
            .iter()
            .find(|c| c.candidate_id == proposal.candidate_ref)
            .ok_or_else(|| CliError::input("review candidate missing"))?;
        // v0.2 claims have no structured provenance field. Preserve the complete
        // reviewed provenance and contradiction links as descriptive claim text.
        let statement = json_out(
            &json!({"historical_assertion":proposal.value,"kind":proposal.kind,
            "source":proposal.source,"source_digest":preview.source_digest,"source_uri":preview.source_uri,
            "candidate_ref":proposal.candidate_ref,"conflicts_with":candidate.conflicts_with,
            "review_digest":reviewed.review.digest,"reviewer":actor_id,
            "not_current_verified_state":true}),
        )?;
        let per_key = StableId::parse(&format!("{key}:{}", proposal.candidate_ref))
            .map_err(CliError::input)?;
        let command = Command::RecordClaim(RecordClaimPayload {
            claim: ClaimRecord {
                schema_version: SchemaVersion::V0_2_0,
                claim_id: StableId::parse(&format!("claim:history:{}", proposal.candidate_ref))
                    .map_err(CliError::input)?,
                statement,
                classification: ClaimClassification::Asserted,
                subject_refs: vec![StableId::parse(project.as_str()).map_err(CliError::input)?],
                evidence_refs: Vec::new(),
                authority_ref: None,
            },
        });
        commands.push(
            prepare_command(CommandDraft {
                schema_version: SchemaVersion::V0_2_0,
                command_id: crate::deterministic_cli_command_id(
                    &format!("legacy:{actor_id}"),
                    &project,
                    &per_key,
                )
                .map_err(CliError::input)?,
                project_ref: project.clone(),
                expected_state_version: initial
                    .checked_add(index as u64)
                    .ok_or_else(|| CliError::input("version overflow"))?,
                idempotency_key: per_key,
                actor: actor.clone(),
                command,
                submitted_at: at.clone(),
            })
            .map_err(CliError::input)?,
        );
    }
    let mut store = SqliteEventStore::open_existing(&database)?;
    let clock = FixedClock::new(at);
    let mut results = Vec::new();
    for command in commands {
        let result =
            CommandProcessor::new(&mut store, &clock).execute(command, &CoreCommandHandler)?;
        let accepted = matches!(
            result.status,
            CommandResultStatus::Accepted | CommandResultStatus::DuplicateReplay
        );
        results.push(result.clone());
        if !accepted {
            let mut error = CliError::command(result).expect_err("nonaccepted result");
            error.result = Some(
                json!({"error":{"code":error.code,"exit_code":error.exit},"partial_results":results,
                "recovery":"reconcile per-candidate keys; replay exact reviewed request, never a new key"}),
            );
            return Err(error);
        }
    }
    output(
        args,
        &json!({"review":reviewed.review,"imported_assertions":results.len(),"results":results,
        "not_materialized":preview.candidates.iter().map(|c| json!({"candidate_ref":c.candidate_id,"kind":c.kind,
            "reason":"historical lifecycle, gates and operational objects are not promoted by v0.2 reviewed import; only explicitly reviewed ASSERTED claims are representable"})).collect::<Vec<_>>(),
        "legacy_import_reviewed_event_available":false}),
    )
}

fn output(args: &LegacyArgs, value: &impl serde::Serialize) -> Result<String, CliError> {
    let bytes = to_canonical_vec(value).map_err(CliError::input)?;
    scan(&bytes)?;
    if let Some(path) = &args.output {
        write_new(path, &bytes)?;
    }
    json_out(value)
}
