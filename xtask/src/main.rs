use std::{env, path::PathBuf, process::ExitCode};

use project_ledger_schemas::{check_generated, write_generated};

mod benchmark;
mod release;

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
    if arguments.len() < 2 {
        return Err(usage());
    }
    match (arguments[0].as_str(), arguments[1].as_str()) {
        ("schema", operation) => {
            let root = parse_root(&arguments[2..])?;
            match operation {
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
                _ => Err(usage()),
            }
        }
        ("benchmark", "run") => {
            let root = parse_root(&arguments[2..])?;
            benchmark::run_and_write(&root)
        }
        ("benchmark", "check") => {
            let root = parse_root(&arguments[2..])?;
            benchmark::check(&root)
        }
        ("release", "generate") => {
            let (root, check) = parse_release_generate(&arguments[2..])?;
            release::generate(&root, check)
        }
        ("release", "check") => {
            let root = parse_root(&arguments[2..])?;
            release::check(&root)
        }
        _ => Err(usage()),
    }
}

fn parse_root(arguments: &[String]) -> Result<PathBuf, String> {
    match arguments {
        [] => env::current_dir().map_err(|error| error.to_string()),
        [flag, path] if flag == "--root" => Ok(PathBuf::from(path)),
        _ => Err("expected no extra arguments or --root <path>".to_owned()),
    }
}

fn parse_release_generate(arguments: &[String]) -> Result<(PathBuf, bool), String> {
    match arguments {
        [flag] if flag == "--check" => {
            Ok((env::current_dir().map_err(|error| error.to_string())?, true))
        }
        [check, root_flag, path] if check == "--check" && root_flag == "--root" => {
            Ok((PathBuf::from(path), true))
        }
        _ => parse_root(arguments).map(|root| (root, false)),
    }
}

fn usage() -> String {
    "usage: cargo xtask schema <generate|check> [--root <path>] | benchmark <run|check> [--root <path>] | release <generate [--check]|check> [--root <path>]".to_owned()
}
