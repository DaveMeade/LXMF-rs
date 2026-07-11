use clap::Parser;
use std::io;
use std::process::Command;

#[derive(Debug, Parser)]
#[command(name = "rnprobe", about = "Probe Reticulum path availability through rnpath")]
struct Cli {
    destination: String,
    #[arg(long, default_value = "127.0.0.1:4243")]
    rpc: String,
    #[arg(long, default_value_t = 30)]
    timeout: u64,
    #[arg(long)]
    json: bool,
}

fn main() -> std::process::ExitCode {
    match probe(&Cli::parse()) {
        Ok(true) => std::process::ExitCode::SUCCESS,
        Ok(false) => std::process::ExitCode::from(2),
        Err(error) => {
            eprintln!("rnprobe: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn probe(cli: &Cli) -> io::Result<bool> {
    let mut command = Command::new("rnpath");
    let timeout = cli.timeout.to_string();
    command.args([cli.destination.as_str(), "--rpc", cli.rpc.as_str(), "--timeout", &timeout]);
    if cli.json {
        command.arg("--json");
    }
    command.status().map(|status| status.success())
}
