use std::{env, path::PathBuf, process::ExitCode};

use project_ledger_schemas::{check_generated, write_generated};

fn main() -> ExitCode {
    match run(env::args().skip(1)) {
        Ok(message) => {
            println!("{message}");
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(arguments: impl Iterator<Item = String>) -> Result<String, String> {
    let arguments: Vec<String> = arguments.collect();
    if arguments.len() < 2 || arguments[0] != "schema" {
        return Err("usage: cargo xtask schema <generate|check> [--root <path>]".to_owned());
    }
    let root = parse_root(&arguments[2..])?;
    match arguments[1].as_str() {
        "generate" => {
            write_generated(&root).map_err(|error| error.to_string())?;
            Ok(format!(
                "generated schema artifacts under {}",
                root.display()
            ))
        }
        "check" => {
            check_generated(&root).map_err(|error| error.to_string())?;
            Ok(format!("schema artifacts match under {}", root.display()))
        }
        _ => Err("usage: cargo xtask schema <generate|check> [--root <path>]".to_owned()),
    }
}

fn parse_root(arguments: &[String]) -> Result<PathBuf, String> {
    match arguments {
        [] => env::current_dir().map_err(|error| error.to_string()),
        [flag, path] if flag == "--root" => Ok(PathBuf::from(path)),
        _ => Err("expected no extra arguments or --root <path>".to_owned()),
    }
}
