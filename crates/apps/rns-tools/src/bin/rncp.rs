use clap::Parser;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Parser)]
#[command(name = "rncp", about = "Copy files through a deterministic Reticulum transfer workflow")]
struct Cli {
    source: PathBuf,
    destination: PathBuf,
    #[arg(long, help = "Restrict both paths to this simulation root")]
    simulate_root: Option<PathBuf>,
    #[arg(long)]
    force: bool,
}

fn main() -> std::process::ExitCode {
    match copy(&Cli::parse()) {
        Ok(bytes) => {
            println!("copied {bytes} bytes");
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("rncp: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn copy(cli: &Cli) -> io::Result<u64> {
    let source = scoped_path(&cli.source, cli.simulate_root.as_deref())?;
    let destination = scoped_path(&cli.destination, cli.simulate_root.as_deref())?;
    if destination.exists() && !cli.force {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "destination exists; use --force",
        ));
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)
}

fn scoped_path(path: &Path, root: Option<&Path>) -> io::Result<PathBuf> {
    if path.components().any(|component| component == Component::ParentDir) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "parent traversal is denied"));
    }
    let Some(root) = root else { return Ok(path.to_path_buf()) };
    let root = root.canonicalize()?;
    let candidate = if path.is_absolute() { path.to_path_buf() } else { root.join(path) };
    let parent = candidate.parent().unwrap_or(&candidate);
    fs::create_dir_all(parent)?;
    let parent = parent.canonicalize()?;
    if !parent.starts_with(&root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "path escapes simulation root",
        ));
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn simulation_copy_is_root_scoped_and_binary_safe() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("source"), [0, 1, 2, 255]).expect("source");
        let cli = Cli {
            source: "source".into(),
            destination: "nested/dest".into(),
            simulate_root: Some(temp.path().into()),
            force: false,
        };
        assert_eq!(copy(&cli).expect("copy"), 4);
        assert_eq!(fs::read(temp.path().join("nested/dest")).expect("dest"), [0, 1, 2, 255]);
    }
}
