use clap::{Parser, Subcommand};
use rand_core::OsRng;
use rns_transport::identity::PrivateIdentity;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Debug, Parser)]
#[command(name = "rnid", about = "Create and inspect Reticulum identities")]
struct Cli {
    #[command(subcommand)]
    command: IdentityCommand,
}

#[derive(Debug, Subcommand)]
enum IdentityCommand {
    Generate {
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        force: bool,
    },
    Show {
        identity: PathBuf,
        #[arg(long)]
        private: bool,
    },
}

fn main() -> std::process::ExitCode {
    match run(Cli::parse()) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("rnid: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> io::Result<()> {
    match cli.command {
        IdentityCommand::Generate { output, force } => {
            if output.exists() && !force {
                return Err(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "identity exists; use --force",
                ));
            }
            let identity = PrivateIdentity::new_from_rand(OsRng);
            write_private_identity(&output, &identity)?;
            println!("{}", identity.address_hash());
        }
        IdentityCommand::Show { identity, private } => {
            let identity = read_private_identity(&identity)?;
            if private {
                println!("{}", hex::encode(identity.to_private_key_bytes()));
            } else {
                println!("{}", identity.address_hash());
            }
        }
    }
    Ok(())
}

fn write_private_identity(path: &Path, identity: &PrivateIdentity) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, identity.to_private_key_bytes())
}

fn read_private_identity(path: &Path) -> io::Result<PrivateIdentity> {
    let bytes = fs::read(path)?;
    PrivateIdentity::from_private_key_bytes(&bytes)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid Reticulum identity"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_file_roundtrips_private_material() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("identity");
        let identity = PrivateIdentity::new_from_rand(OsRng);
        write_private_identity(&path, &identity).expect("write");
        let restored = read_private_identity(&path).expect("read");
        assert_eq!(restored.to_private_key_bytes(), identity.to_private_key_bytes());
    }
}
