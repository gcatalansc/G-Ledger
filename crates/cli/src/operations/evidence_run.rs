use super::{WriteArgs, execute};
use crate::errors::CliError;
use clap::Args;
use project_ledger_application::{CommandProcessor, CoreCommandHandler, FixedClock};
use project_ledger_canonical_json::sha256_digest;
use project_ledger_domain::{
    Command, CommandType, EvidenceMetrics, LedgerTimestamp, RecordVerificationResultPayload,
    ResourceRef, StableId,
};
use project_ledger_evidence::supervisor::{ProcessPolicy, ProcessRequest};
use project_ledger_evidence::{
    EvidenceCommand, EvidenceParser, EvidenceRunContext, EvidenceRunPolicy, EvidenceRunner,
    EvidenceRunnerError, ParsedEvidence,
};
use project_ledger_storage_sqlite::{ExecutionReservation, SqliteEventStore};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeSet, fs, path::PathBuf};

#[derive(Debug, Args)]
pub struct RunArgs {
    #[command(flatten)]
    pub write: WriteArgs,
    #[arg(long)]
    pub policy: PathBuf,
    #[arg(long)]
    pub cwd: PathBuf,
    #[arg(long)]
    pub gate: Option<String>,
    #[arg(long)]
    pub cancel_file: Option<PathBuf>,
    #[arg(last = true, required = true)]
    pub argv: Vec<String>,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    allowed_programs: BTreeSet<String>,
    #[serde(default)]
    redactions: Vec<String>,
    #[serde(default = "summary_limit")]
    max_summary_bytes: usize,
    #[serde(default, skip_serializing_if = "default_process")]
    process: ProcessPolicy,
}
fn default_process(policy: &ProcessPolicy) -> bool {
    *policy == ProcessPolicy::default()
}
const fn summary_limit() -> usize {
    4096
}
struct ObservedParser;
impl EvidenceParser for ObservedParser {
    fn qualified(&self) -> bool {
        false
    }
    fn parse(&self, code: Option<i32>, _stdout: &[u8], _stderr: &[u8]) -> ParsedEvidence {
        ParsedEvidence {
            metrics: EvidenceMetrics {
                passed: None,
                failed: None,
                skipped: None,
                errors: None,
            },
            summary: format!(
                "Process exit: {code:?}; no qualified metrics parser; output not persisted"
            ),
        }
    }
}

#[allow(clippy::too_many_lines)] // Keep authorization, reconciliation, execution and recording in order.
pub fn run(args: &RunArgs) -> Result<String, CliError> {
    let policy: Policy = serde_json::from_slice(&fs::read(&args.policy).map_err(CliError::input)?)
        .map_err(CliError::input)?;
    let (store, projection) = args.write.selection.load()?;
    if !policy.allowed_programs.contains(&args.argv[0]) {
        return Err(CliError::new(
            9,
            "POLICY_DENIED",
            "executable is not authorized by the explicit policy",
        ));
    }
    if let Some(gate) = &args.gate {
        let id = StableId::parse(gate).map_err(CliError::input)?;
        if projection.gate(&id).is_none() {
            return Err(CliError::domain("unknown target gate"));
        }
    }
    let cwd = fs::canonicalize(&args.cwd).map_err(CliError::input)?;
    policy.process.validate(&ProcessRequest {
        program: args.argv[0].clone().into(),
        args: args.argv[1..].to_vec(),
        working_directory: cwd.clone(),
    })?;
    if policy.max_summary_bytes == 0 || args.write.file.is_some() {
        return Err(CliError::input(
            "nonzero summary bound required; evidence run does not accept --file",
        ));
    }
    let kind = serde_json::from_value(json!(args.write.kind.as_deref().unwrap_or("TEST_RESULT")))
        .map_err(CliError::input)?;
    let intent=sha256_digest(&json!({"argv":args.argv,"cwd":cwd,"policy":policy,"kind":args.write.kind,"gate":args.gate})).map_err(CliError::input)?.to_string();
    let actor = StableId::parse(&args.write.actor_id).map_err(CliError::input)?;
    let key = StableId::parse(&args.write.idempotency_key).map_err(CliError::input)?;
    let trust = if args.write.as_human {
        "LOCAL_USER"
    } else {
        "TRUSTED_FOR_RECORDING"
    };
    let project = &projection.state().project.project_id;
    let at = LedgerTimestamp::parse(&args.write.observed_at).map_err(CliError::input)?;
    if let Some(result) = store.reconcile_command(
        project,
        CommandType::RecordVerificationResult,
        &actor,
        trust,
        &key,
    )? {
        let command = store
            .recorded_command(&result.command_ref)?
            .ok_or_else(|| CliError::storage("reconciled command is missing"))?;
        let matching = matches!(&command.command,Command::RecordVerificationResult(p) if p.evidence.source.command_display.as_deref()==Some(intent.as_str()));
        if !matching
            || command.submitted_at != at
            || command.expected_state_version != args.write.expected_version
        {
            return Err(CliError::new(
                10,
                "IDEMPOTENCY_CONFLICT",
                "execution key belongs to a different request",
            ));
        }
        let (path, _) = args.write.selection.resolve()?;
        let mut store = SqliteEventStore::open_existing(&path)?;
        let succeeded = matches!(&command.command,Command::RecordVerificationResult(p) if p.evidence.result.exit_code==Some(0));
        return finish(
            CliError::command(
                CommandProcessor::new(&mut store, &FixedClock::new(at))
                    .execute(command, &CoreCommandHandler)?,
            ),
            succeeded,
        );
    }
    if projection.state().state_version != args.write.expected_version {
        return Err(CliError::new(
            5,
            "VERSION_CONFLICT",
            "execution expected version is stale; process not started",
        ));
    }
    let evidence_id = StableId::parse(&format!(
        "evidence:{}",
        &sha256_digest(&json!([project, actor, trust, key]))
            .map_err(CliError::input)?
            .to_string()[7..]
    ))
    .map_err(CliError::input)?;
    let cancel = crate::process_cancel_token(args.cancel_file.clone())?;
    if cancel.is_cancelled() {
        return Err(project_ledger_evidence::supervisor::SupervisionError::Cancelled.into());
    }
    let scope = sha256_digest(&json!([
        project,
        "RECORD_VERIFICATION_RESULT",
        actor,
        trust,
        key
    ]))
    .map_err(CliError::input)?;
    let request_digest = sha256_digest(&json!([intent, args.write.expected_version, at]))
        .map_err(CliError::input)?;
    let (path, _) = args.write.selection.resolve()?;
    let mut reservation = SqliteEventStore::open_existing(&path)?;
    match reservation.reserve_execution(project, scope, request_digest)? {
        ExecutionReservation::Reserved => {}
        ExecutionReservation::OutcomeUnknown => {
            return Err(CliError::new(
                12,
                "OUTCOME_UNKNOWN",
                "execution was already reserved; reconcile or review it, never rerun blindly",
            ));
        }
        ExecutionReservation::Conflict => {
            return Err(CliError::new(
                10,
                "IDEMPOTENCY_CONFLICT",
                "execution key belongs to a different request",
            ));
        }
    }
    drop(reservation);
    let evidence = EvidenceRunner::run_with_cancel(
        &EvidenceRunPolicy {
            allowed_programs: policy.allowed_programs,
            redactions: policy.redactions,
            max_summary_bytes: policy.max_summary_bytes,
            process: policy.process,
        },
        &EvidenceCommand {
            program: args.argv[0].clone(),
            args: args.argv[1..].to_vec(),
            command_display: intent,
            working_directory: cwd.clone(),
            working_directory_ref: ResourceRef::parse(&format!(
                "urn:project-ledger:cwd:{}",
                sha256_digest(&cwd).map_err(CliError::input)?
            ))
            .map_err(CliError::input)?,
            tool: "explicit-local-executable".to_owned(),
        },
        &EvidenceRunContext {
            evidence_id,
            project_ref: project.clone(),
            kind,
            repository_binding: None,
            observed_at: at,
            provenance_ref: ResourceRef::parse("urn:project-ledger:cli:execution")
                .map_err(CliError::input)?,
            runner_version: env!("CARGO_PKG_VERSION").to_owned(),
        },
        &ObservedParser,
        &cancel,
    )
    .map_err(|error| match error {
        EvidenceRunnerError::PolicyDenied { .. } => CliError::new(9, "POLICY_DENIED", error),
        EvidenceRunnerError::Supervision(error) => error.into(),
        other => CliError::new(12, "OUTCOME_UNKNOWN", other),
    })?;
    let succeeded = evidence.result.exit_code == Some(0);
    finish(
        execute(
            &args.write,
            Command::RecordVerificationResult(RecordVerificationResultPayload { evidence }),
        ),
        succeeded,
    )
}

fn finish(result: Result<String, CliError>, succeeded: bool) -> Result<String, CliError> {
    let output = result?;
    if succeeded {
        return Ok(output);
    }
    let mut error = CliError::new(
        3,
        "EVIDENCE_EXECUTION_FAILED",
        "process failed; its observed evidence was recorded, not approved",
    );
    error.result = Some(serde_json::from_str(&output).map_err(CliError::input)?);
    Err(error)
}
