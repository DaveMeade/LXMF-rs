use clap::{Parser, Subcommand};
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};

#[derive(Debug, Parser)]
#[command(name = "rngit", about = "Run local Git workflows prepared for Reticulum file transport")]
struct Cli {
    #[arg(long)]
    root: PathBuf,
    #[command(subcommand)]
    command: GitCommand,
}

#[derive(Debug, Subcommand)]
enum GitCommand {
    Init {
        path: PathBuf,
    },
    Status {
        path: PathBuf,
    },
    Bundle {
        path: PathBuf,
        output: PathBuf,
        #[arg(default_value = "--all")]
        revision: String,
    },
    Unbundle {
        path: PathBuf,
        bundle: PathBuf,
    },
}

fn main() -> std::process::ExitCode {
    match run(&Cli::parse()) {
        Ok(status) => std::process::ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("rngit: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> io::Result<ExitStatus> {
    let root = cli.root.canonicalize()?;
    match &cli.command {
        GitCommand::Init { path } => git(&root, path, &["init"]),
        GitCommand::Status { path } => git(&root, path, &["status", "--short"]),
        GitCommand::Bundle { path, output, revision } => {
            let output = scoped(&root, output)?;
            git(&root, path, &["bundle", "create", output.to_string_lossy().as_ref(), revision])
        }
        GitCommand::Unbundle { path, bundle } => {
            let bundle = scoped(&root, bundle)?;
            git(&root, path, &["bundle", "unbundle", bundle.to_string_lossy().as_ref()])
        }
    }
}

fn git(root: &Path, repository: &Path, args: &[&str]) -> io::Result<ExitStatus> {
    let repository = scoped(root, repository)?;
    Command::new("git").arg("-C").arg(repository).args(args).status()
}

fn scoped(root: &Path, path: &Path) -> io::Result<PathBuf> {
    if path.components().any(|component| component == std::path::Component::ParentDir) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "parent traversal is denied"));
    }
    let candidate = if path.is_absolute() { path.to_path_buf() } else { root.join(path) };
    if !candidate.starts_with(root) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "path escapes workflow root"));
    }
    Ok(candidate)
}
