use project_ledger_evidence::supervisor::{
    CancelToken, ProcessPolicy, ProcessRequest, SupervisionError, supervise,
};
use std::{
    env, fs,
    io::Write,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

fn run(
    mode: &str,
    policy: &ProcessPolicy,
    token: &CancelToken,
) -> Result<project_ledger_evidence::supervisor::ProcessOutput, SupervisionError> {
    let mut policy = policy.clone();
    policy
        .environment
        .insert("LEDGER_FIXTURE_MODE".into(), mode.into());
    supervise(
        &policy,
        &ProcessRequest {
            program: env::current_exe().unwrap(),
            args: ["--ignored", "--exact", "process_fixture", "--nocapture"]
                .map(str::to_owned)
                .to_vec(),
            working_directory: env::current_dir().unwrap(),
        },
        token,
    )
}

#[test]
fn defaults_are_bounded_and_invalid_limits_are_denied() {
    let mut policy = ProcessPolicy::default();
    assert_eq!(policy.timeout_ms, 60_000);
    assert_eq!(policy.max_stdout_bytes, 1_048_576);
    assert_eq!(policy.max_stderr_bytes, 1_048_576);
    assert!(policy.environment.is_empty());
    policy.timeout_ms = 0;
    assert!(matches!(
        run("ok", &policy, &CancelToken::default()),
        Err(SupervisionError::PolicyDenied)
    ));
}

#[test]
fn hanging_process_times_out_and_cancel_is_effective() {
    let policy = ProcessPolicy {
        timeout_ms: 250,
        ..ProcessPolicy::default()
    };
    let start = Instant::now();
    assert!(matches!(
        run("hang", &policy, &CancelToken::default()),
        Err(SupervisionError::Timeout)
    ));
    assert!(start.elapsed() < Duration::from_secs(4));
    let token = CancelToken::default();
    let requested = token.clone();
    let cancel = thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        requested.cancel();
    });
    assert!(matches!(
        run("hang", &ProcessPolicy::default(), &token),
        Err(SupervisionError::Cancelled)
    ));
    cancel.join().unwrap();
}

#[test]
fn stdout_and_stderr_floods_stop_before_unbounded_capture() {
    let policy = ProcessPolicy {
        timeout_ms: 2000,
        max_stdout_bytes: 4096,
        max_stderr_bytes: 4096,
        ..ProcessPolicy::default()
    };
    for mode in ["stdout", "stderr", "both"] {
        let start = Instant::now();
        let outcome = run(mode, &policy, &CancelToken::default());
        assert!(
            matches!(&outcome, Err(SupervisionError::OutputLimit)),
            "{mode}: {outcome:?}"
        );
        assert!(start.elapsed() < Duration::from_secs(4));
    }
}

#[test]
fn own_descendants_are_cleaned_on_timeout_and_normal_parent_exit() {
    for mode in ["child", "child-exit"] {
        let temp = tempfile::tempdir().unwrap();
        let heartbeat = temp.path().join("heartbeat");
        let mut policy = ProcessPolicy {
            timeout_ms: 1500,
            ..ProcessPolicy::default()
        };
        policy.environment.insert(
            "LEDGER_HEARTBEAT".into(),
            heartbeat.to_str().unwrap().into(),
        );
        let result = run(mode, &policy, &CancelToken::default());
        if mode == "child" {
            assert!(matches!(result, Err(SupervisionError::Timeout)));
        } else {
            assert_eq!(result.unwrap().exit_code, Some(0));
        }
        assert!(heartbeat.exists(), "child must have actually executed");
        let before = fs::read(&heartbeat).unwrap();
        thread::sleep(Duration::from_millis(250));
        assert_eq!(
            before,
            fs::read(&heartbeat).unwrap(),
            "own descendant survived cleanup"
        );
    }
}

#[test]
fn explicit_environment_and_literal_argv_work_and_failure_is_observable() {
    let mut policy = ProcessPolicy::default();
    policy
        .environment
        .insert("LEDGER_LITERAL".into(), "a;b & $(not-a-shell)".into());
    policy
        .environment
        .insert("LEDGER_FIXTURE_MODE".into(), "ok".into());
    let output = supervise(
        &policy,
        &ProcessRequest {
            program: env::current_exe().unwrap(),
            args: [
                "--ignored",
                "--exact",
                "process_fixture",
                "--nocapture",
                "--skip",
                "literal;a & $(not-a-shell)",
            ]
            .map(str::to_owned)
            .to_vec(),
            working_directory: env::current_dir().unwrap(),
        },
        &CancelToken::default(),
    )
    .unwrap();
    assert_eq!(output.exit_code, Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("a;b & $(not-a-shell)"));
    assert!(stdout.contains("literal;a & $(not-a-shell)"));
    assert_eq!(
        run("fail", &policy, &CancelToken::default())
            .unwrap()
            .exit_code,
        Some(17)
    );
}

#[test]
#[ignore = "bounded real-process helper invoked only by supervising tests"]
#[allow(clippy::zombie_processes)] // Deliberately orphan one bounded child to test supervisor cleanup.
fn process_fixture() {
    let mode = env::var("LEDGER_FIXTURE_MODE").unwrap();
    match mode.as_str() {
        "ok" => println!(
            "{} {:?}",
            env::var("LEDGER_LITERAL").unwrap_or_default(),
            env::args().collect::<Vec<_>>()
        ),
        "fail" => std::process::exit(17),
        "stdout" | "stderr" | "both" => {
            for _ in 0..1024 {
                if mode != "stderr" {
                    let _ = std::io::stdout().write_all(&[b'x'; 4096]);
                }
                if mode != "stdout" {
                    let _ = std::io::stderr().write_all(&[b'y'; 4096]);
                }
            }
        }
        "child" | "child-exit" => {
            let mut child = Command::new(env::current_exe().unwrap())
                .args(["--ignored", "--exact", "process_fixture", "--nocapture"])
                .env("LEDGER_FIXTURE_MODE", "heartbeat")
                .stdin(Stdio::null())
                .spawn()
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            while !std::path::Path::new(&env::var("LEDGER_HEARTBEAT").unwrap()).exists()
                && Instant::now() < deadline
            {
                thread::sleep(Duration::from_millis(10));
            }
            if mode == "child-exit" {
                return;
            }
            let _ = child.wait();
        }
        "heartbeat" => {
            for index in 0..600 {
                if env::var_os("LEDGER_HEARTBEAT_STOP")
                    .is_some_and(|path| std::path::Path::new(&path).exists())
                {
                    break;
                }
                fs::write(env::var("LEDGER_HEARTBEAT").unwrap(), index.to_string()).unwrap();
                thread::sleep(Duration::from_millis(20));
            }
        }
        "hang" => thread::sleep(Duration::from_secs(12)),
        _ => panic!("unknown fixture mode"),
    }
}

#[test]
#[cfg(windows)]
fn windows_closes_the_owned_job_when_the_supervisor_is_terminated() {
    let temp = tempfile::tempdir().unwrap();
    let heartbeat = temp.path().join("forced-heartbeat");
    let stop = temp.path().join("stop-fixture");
    let mut host = Command::new(env::current_exe().unwrap())
        .args([
            "--ignored",
            "--exact",
            "supervisor_host_fixture",
            "--nocapture",
        ])
        .env("LEDGER_HEARTBEAT", &heartbeat)
        .env("LEDGER_HEARTBEAT_STOP", &stop)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    while !heartbeat.exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    host.kill().unwrap();
    host.wait().unwrap();
    thread::sleep(Duration::from_millis(100));
    let before = fs::read(&heartbeat);
    thread::sleep(Duration::from_millis(150));
    let after = fs::read(&heartbeat);
    // Even the RED implementation leaves no unbounded helper running.
    fs::write(&stop, b"stop").unwrap();
    assert_eq!(
        before.unwrap(),
        after.unwrap(),
        "supervisor termination leaked its own descendant"
    );
}

#[test]
#[cfg(windows)]
#[ignore = "real host fixture terminated by its owning test"]
fn supervisor_host_fixture() {
    let mut policy = ProcessPolicy {
        timeout_ms: 8000,
        ..ProcessPolicy::default()
    };
    for key in ["LEDGER_HEARTBEAT", "LEDGER_HEARTBEAT_STOP"] {
        policy
            .environment
            .insert(key.into(), env::var(key).unwrap());
    }
    let _ = run("child", &policy, &CancelToken::default());
}
