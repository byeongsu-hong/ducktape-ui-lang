mod compat;
mod lsp;
mod schema;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.first().is_some_and(|arg| arg == "ice") {
        args.remove(0);
    }
    let command = args.first().map(String::as_str).unwrap_or("check");
    let check_only = args.iter().any(|arg| arg == "--check");

    match command {
        "schema" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&schema::document())
                    .map_err(|error| error.to_string())?
            );
            return Ok(());
        }
        "lsp" => return lsp::run_stdio(),
        "help" | "--help" | "-h" => {
            println!(
                "cargo ice <fmt [--check] | check | clippy | compat | expand <file.ice> | schema | lsp>"
            );
            return Ok(());
        }
        _ => {}
    }

    let root = env::current_dir().map_err(|error| error.to_string())?;
    let files = ice_files(&root)?;

    match command {
        "fmt" => {
            let roots = app_files(&files)?;
            if check_only {
                cargo(&["fmt", "--all", "--", "--check"])?;
            } else {
                cargo(&["fmt", "--all"])?;
            }
            let mut changed = Vec::new();
            for path in &files {
                let source = fs::read_to_string(path).map_err(|error| error.to_string())?;
                let formatted = ui_lang_core::format_fragment(&source);
                if source != formatted {
                    changed.push(path.display().to_string());
                    if !check_only {
                        fs::write(path, formatted).map_err(|error| error.to_string())?;
                    }
                }
            }
            if check_only && !changed.is_empty() {
                return Err(format!("unformatted .ice files:\n{}", changed.join("\n")));
            }
            analyze(&roots)?;
            if check_only {
                println!("formatting is clean for {} .ice file(s)", files.len());
            } else {
                println!("formatted {} .ice file(s)", files.len());
            }
        }
        "check" => {
            let roots = app_files(&files)?;
            analyze(&roots)?;
            cargo(&["check", "--workspace"])?;
        }
        "clippy" => {
            let roots = app_files(&files)?;
            analyze(&roots)?;
            cargo(&["clippy", "--workspace", "--all-targets", "--no-deps"])?;
        }
        "compat" => {
            let roots = app_files(&files)?;
            analyze(&roots)?;
            compat::verify(&root)?;
            cargo(&["test", "-p", "iced-app"])?;
        }
        "expand" => {
            let requested = args
                .get(1)
                .ok_or_else(|| "cargo ice expand <file.ice>".to_owned())?;
            let path = root.join(requested);
            let generated = ui_lang_core::compile_file(&path)
                .map_err(|error| error.render(&path.display().to_string()))?;
            print!("{}", generated.rust);
        }
        other => return Err(format!("unknown cargo ice command `{other}`")),
    }
    Ok(())
}

fn analyze(files: &[PathBuf]) -> Result<(), String> {
    for path in files {
        ui_lang_core::analyze_file(path)
            .map_err(|error| error.render(&path.display().to_string()))?;
    }
    println!("checked {} .ice app graph(s)", files.len());
    Ok(())
}

fn app_files(files: &[PathBuf]) -> Result<Vec<PathBuf>, String> {
    let mut roots = Vec::new();
    for path in files {
        let source = fs::read_to_string(path).map_err(|error| error.to_string())?;
        if ui_lang_core::source_is_app(&source) {
            roots.push(path.clone());
        }
    }
    if roots.is_empty() {
        return Err("no .ice file contains a top-level `app` declaration".into());
    }
    Ok(roots)
}

fn ice_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    fn visit(path: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                if !ignored_dir(&path) {
                    visit(&path, output)?;
                }
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("ice") {
                output.push(path);
            }
        }
        Ok(())
    }

    let mut output = Vec::new();
    visit(root, &mut output)?;
    output.sort();
    Ok(output)
}

fn ignored_dir(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(".git" | ".worktree" | "target")
    ) || (path.file_name().and_then(|name| name.to_str()) == Some("cases")
        && path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("tests"))
}

fn cargo(args: &[&str]) -> Result<(), String> {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let status = Command::new(cargo)
        .args(args)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("cargo {} failed", args.join(" ")))
    }
}

#[cfg(test)]
mod tests {
    use super::ignored_dir;
    use std::path::Path;

    #[test]
    fn ignores_build_and_fixture_directories() {
        assert!(ignored_dir(Path::new("target")));
        assert!(ignored_dir(Path::new(".worktree")));
        assert!(ignored_dir(Path::new("tests/cases")));
        assert!(!ignored_dir(Path::new("src/cases")));
    }
}
