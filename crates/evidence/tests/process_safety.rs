use project_ledger_domain::{
    EvidenceKind, EvidenceMetrics, LedgerTimestamp, ProjectId, ResourceRef, StableId,
};
use project_ledger_evidence::{
    EvidenceCommand, EvidenceParser, EvidenceRunContext, EvidenceRunPolicy, EvidenceRunner,
    EvidenceRunnerError, ParsedEvidence,
};
use std::{collections::BTreeSet, env, process::Command};

struct OutputParser;
impl EvidenceParser for OutputParser {
    fn qualified(&self) -> bool {
        false
    }
    fn parse(&self, _: Option<i32>, stdout: &[u8], _: &[u8]) -> ParsedEvidence {
        ParsedEvidence {
            metrics: EvidenceMetrics {
                passed: None,
                failed: None,
                skipped: None,
                errors: None,
            },
            summary: String::from_utf8_lossy(stdout).into_owned(),
        }
    }
}
fn context() -> EvidenceRunContext {
    EvidenceRunContext {
        evidence_id: StableId::parse("evidence:process-safety").unwrap(),
        project_ref: ProjectId::parse("project:0198f8c8-7417-7fb0-b88f-0fe0c0a7df5d").unwrap(),
        kind: EvidenceKind::TestResult,
        repository_binding: None,
        observed_at: LedgerTimestamp::parse("2026-08-27T12:00:00Z").unwrap(),
        provenance_ref: ResourceRef::parse("urn:test:process-safety").unwrap(),
        runner_version: "test/1".into(),
    }
}
fn request(program: String, args: Vec<String>) -> EvidenceCommand {
    EvidenceCommand {
        program,
        args,
        command_display: "explicit test program".into(),
        working_directory: env::current_dir().unwrap(),
        working_directory_ref: ResourceRef::parse("urn:test:cwd").unwrap(),
        tool: "test-process".into(),
    }
}
fn policy(program: String) -> EvidenceRunPolicy {
    EvidenceRunPolicy {
        allowed_programs: BTreeSet::from([program]),
        redactions: vec![],
        max_summary_bytes: 4096,
        ..EvidenceRunPolicy::default()
    }
}

#[test]
fn parent_credentials_are_not_inherited_by_evidence_processes() {
    let canary = "LEDGER_FICTITIOUS_CREDENTIAL_734b";
    let out = Command::new(env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "run_environment_probe",
            "--nocapture",
        ])
        .env("LEDGER_PRIVATE_CANARY", canary)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains(canary),
        "parent credential leaked into the persisted summary"
    );
}

#[test]
fn relative_executable_is_denied_without_searching_parent_path() {
    let request = request("rustc".into(), vec!["--version".into()]);
    assert!(matches!(
        EvidenceRunner::run(
            &policy(request.program.clone()),
            &request,
            &context(),
            &OutputParser
        ),
        Err(EvidenceRunnerError::PolicyDenied { .. })
    ));
}

#[test]
#[ignore = "real child fixture, invoked explicitly by its supervising test"]
fn run_environment_probe() {
    let exe = env::current_exe().unwrap().to_str().unwrap().to_owned();
    let request = request(
        exe.clone(),
        [
            "--ignored",
            "--exact",
            "print_inherited_canary",
            "--nocapture",
        ]
        .map(str::to_owned)
        .to_vec(),
    );
    let record = EvidenceRunner::run(&policy(exe), &request, &context(), &OutputParser).unwrap();
    println!("{}", serde_json::to_string(&record).unwrap());
}

#[test]
#[ignore = "real child fixture, invoked explicitly by its supervising test"]
fn print_inherited_canary() {
    println!(
        "{}",
        env::var("LEDGER_PRIVATE_CANARY").unwrap_or_else(|_| "NO_PARENT_CREDENTIAL".into())
    );
}

#[test]
fn summaries_are_utf8_bounded_and_supported_secret_displays_are_redacted() {
    let exe = env::current_exe().unwrap().to_str().unwrap().to_owned();
    let mut request = request(
        exe.clone(),
        ["--ignored", "--exact", "print_long_summary", "--nocapture"]
            .map(str::to_owned)
            .to_vec(),
    );
    request.command_display = "api_key=FICTITIOUS_UNCONFIGURED_CANARY".into();
    let mut policy = policy(exe);
    policy.max_summary_bytes = 95;
    let record = EvidenceRunner::run(&policy, &request, &context(), &OutputParser).unwrap();
    assert!(record.result.summary.len() <= 95);
    assert_eq!(record.source.command_display.as_deref(), Some("[REDACTED]"));
    assert_eq!(
        record.status,
        project_ledger_domain::EvidenceStatus::Observed
    );
    assert!(!record.privacy.raw_output_stored);
}

#[test]
#[ignore = "bounded UTF8 process-output fixture"]
fn print_long_summary() {
    println!("{}", "á🦀".repeat(1000));
}
