use clap::Parser;
use std::collections::BTreeSet;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};

#[derive(Debug, Parser)]
#[command(name = "rnsh", about = "Run an authorised command in a Reticulum shell workflow")]
struct Cli {
    #[arg(long)]
    root: PathBuf,
    #[arg(long = "allow", required = true)]
    allowed: Vec<String>,
    #[arg(required = true, trailing_var_arg = true)]
    command: Vec<String>,
}

fn main() -> std::process::ExitCode {
    match execute(&Cli::parse()) {
        Ok(status) => std::process::ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(error) => {
            eprintln!("rnsh: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn execute(cli: &Cli) -> io::Result<ExitStatus> {
    let executable = cli
        .command
        .first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "command required"))?;
    let allowed = cli.allowed.iter().cloned().collect::<BTreeSet<_>>();
    if !allowed.contains(executable) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "command denied by default-deny policy",
        ));
    }
    let root = cli.root.canonicalize()?;
    Command::new(executable).args(&cli.command[1..]).current_dir(root).status()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_policy_denies_unlisted_command() {
        let temp = tempfile::tempdir().expect("tempdir");
        let cli = Cli {
            root: temp.path().into(),
            allowed: vec!["true".into()],
            command: vec!["false".into()],
        };
        assert_eq!(execute(&cli).expect_err("denied").kind(), io::ErrorKind::PermissionDenied);
    }
}
