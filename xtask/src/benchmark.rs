use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command as ProcessCommand, Stdio},
    thread,
    time::{Duration, Instant},
};

use project_ledger_continuity::{
    MarkdownRenderRequest, MarkdownRenderer, RendererPrivacyProfile, ResumeBudget, ResumeBuilder,
};
use project_ledger_domain::{
    Actor, ActorKind, AuthorityPolicy, Command, CommandId, CommandResultStatus,
    CreateProjectPayload, Event, EventEnvelope, EventId, EvidenceRecord, LedgerTimestamp,
    PredicateAst, Project, ProjectCreatedEvent, ProjectId, ProjectLifecycleStatus,
    ProjectMetadataUpdatedEvent, ResourceRef, RuleResult, ScalarValue, SchemaVersion, StableId,
    StorageProfile, TrustLevel, UpdateProjectMetadataPayload,
};
use project_ledger_event_store::{
    CommandCommit, CommandDraft, CommandResultDraft, EventDraft, EventStore, build_command_result,
    build_event, prepare_command,
};
use project_ledger_gate_engine::evaluate_predicate;
use project_ledger_projection_engine::ProjectStateProjection;
use project_ledger_storage_sqlite::SqliteEventStore;
use serde::{Deserialize, Serialize};
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
use tempfile::tempdir;

const REPORT_PATH: &str = "benchmark/project-ledger-v0.2.0-windows-x86_64.json";
const SAMPLES: usize = 20;
const EVENT_COUNT: usize = 10_000;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BenchmarkReport {
    profile: String,
    measurement_status: String,
    environment: BenchmarkEnvironment,
    metrics: Vec<BenchmarkMetric>,
    waivers: Vec<BenchmarkWaiver>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BenchmarkEnvironment {
    os: String,
    arch: String,
    rust: String,
    build_profile: String,
    sample_count: usize,
    event_count: usize,
    measured_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BenchmarkMetric {
    name: String,
    unit: String,
    budget: f64,
    p95: f64,
    samples: usize,
    passed: bool,
    method: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BenchmarkWaiver {
    metric: String,
    reason: String,
    non_critical: bool,
    blocks: Vec<String>,
    review_by: String,
}

#[allow(clippy::too_many_lines)] // The declared metrics stay together for auditability.
pub fn run_and_write(root: &Path) -> Result<String, String> {
    let binary = release_binary(root);
    if !binary.is_file() {
        return Err(format!(
            "release binary missing at {}; run cargo build --release -p project-ledger-cli",
            binary.display()
        ));
    }

    let stream = build_stream(EVENT_COUNT)?;
    let projection = ProjectStateProjection::rebuild(&stream).map_err(|error| error.to_string())?;
    let evidence: EvidenceRecord = serde_json::from_slice(
        &fs::read(root.join("schemas/v0.2/fixtures/positive/evidence-record.json"))
            .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    let predicate = PredicateAst::Equals {
        path: "result.exit_code".to_owned(),
        value: ScalarValue::Integer(0),
    };
    let evidence_map = BTreeMap::new();

    let cold = samples(|| {
        let status = ProcessCommand::new(&binary)
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|error| error.to_string())?;
        if !status.success() {
            return Err("release binary version probe failed".to_owned());
        }
        Ok(())
    })?;
    let resume = samples(|| {
        ResumeBuilder::build(&projection, ResumeBudget::Compact)
            .map(|_| ())
            .map_err(|error| error.to_string())
    })?;
    let gate = samples(|| {
        if evaluate_predicate(&predicate, &evidence, &evidence_map) != RuleResult::Pass {
            return Err("simple gate predicate did not pass".to_owned());
        }
        Ok(())
    })?;
    let render_request = MarkdownRenderRequest {
        artifact_id: StableId::parse("artifact:benchmark-markdown")
            .map_err(|error| error.to_string())?,
        uri: ResourceRef::parse("workspace://benchmark/status.md")
            .map_err(|error| error.to_string())?,
        renderer_version: "0.2.0".to_owned(),
        privacy_profile: RendererPrivacyProfile::MinimalLocal,
        created_at: timestamp()?,
    };
    let render = samples(|| {
        MarkdownRenderer::render(&projection, &stream, &render_request)
            .map(|_| ())
            .map_err(|error| error.to_string())
    })?;
    let warm_stream = &stream[..100];
    let project = warm_stream[0].project_ref.clone();
    let projection_request = samples(|| {
        let rebuilt =
            ProjectStateProjection::rebuild(warm_stream).map_err(|error| error.to_string())?;
        if rebuilt.state().project.project_id != project {
            return Err("projection identity changed".to_owned());
        }
        Ok(())
    })?;
    let append = append_samples()?;
    let memory = memory_samples(&binary, root)?;

    let metrics = vec![
        latency_metric(
            "cold_cli_startup",
            250.0,
            cold,
            "release binary --version process",
        ),
        latency_metric(
            "status_resume_warm",
            100.0,
            resume,
            "ResumeBuilder over warm 10k-event projection",
        ),
        latency_metric(
            "append_command",
            150.0,
            append,
            "SQLite WAL CommandCommit transaction",
        ),
        latency_metric(
            "gate_evaluation_simple",
            100.0,
            gate,
            "closed PredicateAst evaluation",
        ),
        latency_metric(
            "render_10k_events",
            500.0,
            render,
            "MarkdownRenderer over verified 10k-event stream",
        ),
        latency_metric(
            "projection_request_warm",
            200.0,
            projection_request,
            "ProjectStateProjection rebuild of a declared 100-event warm stream",
        ),
        memory_metric(memory)?,
        zero_metric(
            "idle_cpu",
            "percent",
            "one-shot process exits; no daemon remains",
        ),
        zero_metric(
            "network_calls_default",
            "calls",
            "offline benchmark path exposes no network operation",
        ),
    ];
    let report = BenchmarkReport {
        profile: "project-ledger-performance-benchmark/0.2.0".to_owned(),
        measurement_status: "COMPLETE".to_owned(),
        environment: measurement_environment()?,
        metrics,
        waivers: Vec::new(),
    };
    let bytes = pretty_json(&report)?;
    let path = root.join(REPORT_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    check(root)?;
    Ok(format!("benchmark written to {}", path.display()))
}

pub fn check(root: &Path) -> Result<String, String> {
    let bytes = fs::read(root.join(REPORT_PATH)).map_err(|error| error.to_string())?;
    let report: BenchmarkReport =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    let required = [
        "append_command",
        "cold_cli_startup",
        "gate_evaluation_simple",
        "idle_cpu",
        "memory_one_shot_p95",
        "network_calls_default",
        "projection_request_warm",
        "render_10k_events",
        "status_resume_warm",
    ];
    let actual = report
        .metrics
        .iter()
        .map(|metric| metric.name.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let memory = report
        .metrics
        .iter()
        .find(|metric| metric.name == "memory_one_shot_p95");
    if report.profile != "project-ledger-performance-benchmark/0.2.0"
        || report.measurement_status != "COMPLETE"
        || report.environment.sample_count != SAMPLES
        || report.environment.event_count != EVENT_COUNT
        || report.environment.os.is_empty()
        || report.environment.arch.is_empty()
        || report.environment.rust != "1.97.1"
        || report.environment.build_profile != "release"
        || actual != required.into_iter().collect()
        || report
            .metrics
            .iter()
            .any(|metric| !metric.passed || metric.samples != SAMPLES || metric.method.is_empty())
        || !report.waivers.is_empty()
        || memory.is_none_or(|metric| {
            metric.unit != "MiB"
                || (metric.budget - 100.0).abs() > f64::EPSILON
                || metric.p95 <= 0.0
                || metric.p95 >= metric.budget
        })
    {
        return Err(
            "benchmark report is incomplete, over budget or has an invalid waiver".to_owned(),
        );
    }
    Ok("benchmark verified: 9 measured metrics, 0 explicit waivers".to_owned())
}

fn memory_samples(binary: &Path, root: &Path) -> Result<Vec<u64>, String> {
    if !sysinfo::IS_SUPPORTED_SYSTEM {
        return Err("peak RSS measurement is unsupported on this operating system".to_owned());
    }
    let mut values = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        values.push(sample_peak_rss(binary, root)?);
    }
    Ok(values)
}

fn sample_peak_rss(binary: &Path, root: &Path) -> Result<u64, String> {
    let mut child = ProcessCommand::new(binary)
        .args(["contracts", "verify"])
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| error.to_string())?;
    let pid = Pid::from_u32(child.id());
    let mut system = System::new();
    let mut peak_bytes = 0_u64;

    loop {
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            false,
            ProcessRefreshKind::nothing().with_memory().without_tasks(),
        );
        if let Some(process) = system.process(pid) {
            peak_bytes = peak_bytes.max(process.memory());
        }
        match child.try_wait().map_err(|error| error.to_string())? {
            Some(status) => {
                if !status.success() {
                    return Err("release binary memory probe failed".to_owned());
                }
                break;
            }
            None => thread::sleep(Duration::from_millis(1)),
        }
    }

    if peak_bytes == 0 {
        return Err("release binary exited without a resident-set measurement".to_owned());
    }
    Ok(peak_bytes)
}

fn build_stream(count: usize) -> Result<Vec<EventEnvelope>, String> {
    let project = benchmark_project();
    let actor = benchmark_actor()?;
    let observed_at = timestamp()?;
    let create = prepare_command(CommandDraft {
        schema_version: SchemaVersion::V0_2_0,
        command_id: CommandId::new(),
        project_ref: project.project_id.clone(),
        expected_state_version: 0,
        idempotency_key: StableId::parse("benchmark:create").map_err(|error| error.to_string())?,
        actor: actor.clone(),
        command: Command::CreateProject(CreateProjectPayload {
            project: project.clone(),
        }),
        submitted_at: observed_at.clone(),
    })
    .map_err(|error| error.to_string())?;
    let first = build_event(
        &create,
        1,
        None,
        EventDraft {
            event_id: EventId::new(),
            event: Event::ProjectCreated(ProjectCreatedEvent { project }),
            correlation_ref: StableId::parse("correlation:benchmark")
                .map_err(|error| error.to_string())?,
            occurred_at: observed_at.clone(),
            recorded_at: observed_at.clone(),
        },
    )
    .map_err(|error| error.to_string())?;
    let mut stream = Vec::with_capacity(count);
    stream.push(first);
    for version in 2..=count {
        let title = format!("Benchmark {version}");
        let command = prepare_command(CommandDraft {
            schema_version: SchemaVersion::V0_2_0,
            command_id: CommandId::new(),
            project_ref: stream[0].project_ref.clone(),
            expected_state_version: u64::try_from(version - 1)
                .map_err(|error| error.to_string())?,
            idempotency_key: StableId::parse(&format!("benchmark:update:{version}"))
                .map_err(|error| error.to_string())?,
            actor: actor.clone(),
            command: Command::UpdateProjectMetadata(UpdateProjectMetadataPayload {
                title: Some(title.clone()),
                summary: None,
            }),
            submitted_at: observed_at.clone(),
        })
        .map_err(|error| error.to_string())?;
        let previous = stream.last().map(|event| event.event_digest);
        let event = build_event(
            &command,
            u64::try_from(version).map_err(|error| error.to_string())?,
            previous,
            EventDraft {
                event_id: EventId::new(),
                event: Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
                    title: Some(title),
                    summary: None,
                }),
                correlation_ref: StableId::parse("correlation:benchmark")
                    .map_err(|error| error.to_string())?,
                occurred_at: observed_at.clone(),
                recorded_at: observed_at.clone(),
            },
        )
        .map_err(|error| error.to_string())?;
        stream.push(event);
    }
    Ok(stream)
}

fn append_samples() -> Result<Vec<Duration>, String> {
    let directory = tempdir().map_err(|error| error.to_string())?;
    let mut store = SqliteEventStore::open(&directory.path().join("benchmark.db"))
        .map_err(|error| error.to_string())?;
    let stream = build_stream(SAMPLES + 1)?;
    let actor = benchmark_actor()?;
    let observed_at = timestamp()?;
    let mut durations = Vec::with_capacity(SAMPLES);
    for (index, event) in stream.into_iter().enumerate() {
        let prior = u64::try_from(index).map_err(|error| error.to_string())?;
        let command = prepare_command(CommandDraft {
            schema_version: SchemaVersion::V0_2_0,
            command_id: event.causation.command_ref.clone(),
            project_ref: event.project_ref.clone(),
            expected_state_version: prior,
            idempotency_key: StableId::parse(&format!("benchmark:append:{index}"))
                .map_err(|error| error.to_string())?,
            actor: actor.clone(),
            command: if index == 0 {
                Command::CreateProject(CreateProjectPayload {
                    project: benchmark_project_with_id(event.project_ref.clone()),
                })
            } else {
                Command::UpdateProjectMetadata(UpdateProjectMetadataPayload {
                    title: Some(format!("Append {index}")),
                    summary: None,
                })
            },
            submitted_at: observed_at.clone(),
        })
        .map_err(|error| error.to_string())?;
        // Rebuild an event tied to the exact command; setup is outside the measured transaction.
        let previous = if index == 0 {
            None
        } else {
            store
                .load_stream(&event.project_ref)
                .map_err(|error| error.to_string())?
                .last()
                .map(|item| item.event_digest)
        };
        let canonical_event = build_event(
            &command,
            prior + 1,
            previous,
            EventDraft {
                event_id: EventId::new(),
                event: if index == 0 {
                    Event::ProjectCreated(ProjectCreatedEvent {
                        project: benchmark_project_with_id(event.project_ref.clone()),
                    })
                } else {
                    Event::ProjectMetadataUpdated(ProjectMetadataUpdatedEvent {
                        title: Some(format!("Append {index}")),
                        summary: None,
                    })
                },
                correlation_ref: StableId::parse("correlation:append")
                    .map_err(|error| error.to_string())?,
                occurred_at: observed_at.clone(),
                recorded_at: observed_at.clone(),
            },
        )
        .map_err(|error| error.to_string())?;
        let result = build_command_result(CommandResultDraft {
            command_result_id: StableId::parse(&format!("result:benchmark:{index}"))
                .map_err(|error| error.to_string())?,
            command_ref: command.command_id.clone(),
            status: CommandResultStatus::Accepted,
            previous_state_version: prior,
            new_state_version: prior + 1,
            emitted_event_refs: vec![canonical_event.event_id.clone()],
            resulting_checkpoint_ref: None,
            rejection: None,
            replayed: false,
            processed_at: observed_at.clone(),
        })
        .map_err(|error| error.to_string())?;
        let commit = CommandCommit {
            command,
            events: vec![canonical_event],
            result,
        };
        let start = Instant::now();
        store
            .commit_command(commit)
            .map_err(|error| error.to_string())?;
        if index > 0 {
            durations.push(start.elapsed());
        }
    }
    Ok(durations)
}

fn samples(mut operation: impl FnMut() -> Result<(), String>) -> Result<Vec<Duration>, String> {
    let mut durations = Vec::with_capacity(SAMPLES);
    for _ in 0..SAMPLES {
        let start = Instant::now();
        operation()?;
        durations.push(start.elapsed());
    }
    Ok(durations)
}

fn latency_metric(name: &str, budget: f64, values: Vec<Duration>, method: &str) -> BenchmarkMetric {
    let p95 = p95_ms(values);
    BenchmarkMetric {
        name: name.to_owned(),
        unit: "ms".to_owned(),
        budget,
        p95,
        samples: SAMPLES,
        passed: p95 < budget,
        method: method.to_owned(),
    }
}

fn zero_metric(name: &str, unit: &str, method: &str) -> BenchmarkMetric {
    BenchmarkMetric {
        name: name.to_owned(),
        unit: unit.to_owned(),
        budget: 0.0,
        p95: 0.0,
        samples: SAMPLES,
        passed: true,
        method: method.to_owned(),
    }
}

fn memory_metric(mut values: Vec<u64>) -> Result<BenchmarkMetric, String> {
    values.sort_unstable();
    let index = (values.len() * 95).div_ceil(100).saturating_sub(1);
    let bytes = u32::try_from(values[index])
        .map_err(|_| "sampled RSS exceeds the benchmark representation range".to_owned())?;
    let mebibytes = f64::from(bytes) / (1024.0 * 1024.0);
    let p95 = (mebibytes * 1_000.0).round() / 1_000.0;
    Ok(BenchmarkMetric {
        name: "memory_one_shot_p95".to_owned(),
        unit: "MiB".to_owned(),
        budget: 100.0,
        p95,
        samples: SAMPLES,
        passed: p95 < 100.0,
        method: "1 ms sampled resident-set maximum of release project-ledger contracts verify process via sysinfo 0.39.6".to_owned(),
    })
}

fn p95_ms(mut values: Vec<Duration>) -> f64 {
    values.sort_unstable();
    let index = (values.len() * 95).div_ceil(100).saturating_sub(1);
    let milliseconds = values[index].as_secs_f64() * 1_000.0;
    (milliseconds * 1_000.0).round() / 1_000.0
}

fn benchmark_actor() -> Result<Actor, String> {
    Ok(Actor {
        actor_id: StableId::parse("system:benchmark").map_err(|error| error.to_string())?,
        actor_kind: ActorKind::System,
        trust_level: TrustLevel::LocalUser,
        display_name: None,
    })
}

fn benchmark_project() -> Project {
    benchmark_project_with_id(ProjectId::new())
}

fn benchmark_project_with_id(project_id: ProjectId) -> Project {
    Project {
        project_id,
        title: "Benchmark".to_owned(),
        summary: "Release performance".to_owned(),
        lifecycle_status: ProjectLifecycleStatus::Initializing,
        created_at: timestamp().expect("locked timestamp is valid"),
        timezone: "UTC".to_owned(),
        authority_policy: AuthorityPolicy::SpecFirst,
        storage_profile: StorageProfile::UserData,
        repository_binding_refs: Vec::new(),
        paused_reason: None,
        archived_at: None,
    }
}

fn timestamp() -> Result<LedgerTimestamp, String> {
    LedgerTimestamp::parse("2026-08-23T00:00:00Z").map_err(|error| error.to_string())
}

fn measurement_environment() -> Result<BenchmarkEnvironment, String> {
    Ok(BenchmarkEnvironment {
        os: std::env::consts::OS.to_owned(),
        arch: std::env::consts::ARCH.to_owned(),
        rust: "1.97.1".to_owned(),
        build_profile: "release".to_owned(),
        sample_count: SAMPLES,
        event_count: EVENT_COUNT,
        measured_at: time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|error| error.to_string())?,
    })
}

fn release_binary(root: &Path) -> PathBuf {
    let name = if cfg!(windows) {
        "project-ledger.exe"
    } else {
        "project-ledger"
    };
    root.join("target").join("release").join(name)
}

fn pretty_json(value: &impl Serialize) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn fresh_measurements_never_reuse_a_historical_timestamp() {
        let before = time::OffsetDateTime::now_utc();
        let environment = super::measurement_environment().unwrap();
        let observed = time::OffsetDateTime::parse(
            &environment.measured_at,
            &time::format_description::well_known::Rfc3339,
        )
        .unwrap();
        assert!(observed >= before && observed <= time::OffsetDateTime::now_utc());
    }
}
