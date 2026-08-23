use std::{env, fmt::Write as _, fs, path::Path, process::ExitCode};

use clap::{Parser, Subcommand};
use project_ledger_canonical_json::sha256_bytes;
use project_ledger_schemas::{check_generated, validate_contracts};

const V0_1_DIGEST: &str = "sha256:772ea51780a5ed2881a786a198aa90dedb8b4ba619b6652d8f54ef4ca306d865";
const V0_2_DIGEST: &str = "sha256:31cb7cd450a6a12d2d92f5ecb018fbf23fd822a20816b2b018db6197bd404dd9";

#[derive(Debug, Parser)]
#[command(
    name = "project-ledger",
    version,
    about = "Verifiable local project continuity"
)]
struct Cli {
    #[command(subcommand)]
    command: TopLevelCommand,
}

#[derive(Debug, Subcommand)]
enum TopLevelCommand {
    Specs {
        #[command(subcommand)]
        command: VerificationCommand,
    },
    Contracts {
        #[command(subcommand)]
        command: VerificationCommand,
    },
}

#[derive(Clone, Copy, Debug, Subcommand)]
enum VerificationCommand {
    Verify,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = match env::current_dir() {
        Ok(root) => root,
        Err(error) => {
            eprintln!("cannot determine repository root: {error}");
            return ExitCode::FAILURE;
        }
    };
    let result = match cli.command {
        TopLevelCommand::Specs {
            command: VerificationCommand::Verify,
        } => verify_specs(&root),
        TopLevelCommand::Contracts {
            command: VerificationCommand::Verify,
        } => verify_contract_artifacts(&root),
    };
    match result {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn verify_specs(root: &Path) -> Result<String, String> {
    let versions = [
        (
            "v0.1.0",
            root.join("specs/history/PROJECT_LEDGER_SPEC_v0.1.0.md"),
            V0_1_DIGEST,
        ),
        (
            "v0.2.0",
            root.join("specs/PROJECT_LEDGER_SPEC_v0.2.0.md"),
            V0_2_DIGEST,
        ),
    ];
    let mut output = String::new();
    for (version, path, expected) in versions {
        let bytes = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let actual = sha256_bytes(&bytes).to_string();
        if actual != expected {
            return Err(format!(
                "{version} MISMATCH expected={expected} actual={actual}"
            ));
        }
        writeln!(&mut output, "{version} MATCH {}", &actual[7..15])
            .map_err(|error| error.to_string())?;
    }
    Ok(output)
}

fn verify_contract_artifacts(root: &Path) -> Result<String, String> {
    check_generated(root).map_err(|error| error.to_string())?;
    let report = validate_contracts().map_err(|error| error.to_string())?;
    Ok(format!(
        "contracts MATCH schemas={} fixtures={} matrix_rows={}\n",
        report.schema_count,
        report.positive_fixture_count + report.negative_fixture_count,
        report.matrix_row_count
    ))
}
