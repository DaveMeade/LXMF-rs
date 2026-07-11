use clap::{Arg, ArgAction, Command};

fn main() {
    let matches = Command::new("rnir")
        .about("Reticulum Distributed Identity Resolver")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::new("config").long("config").value_name("PATH"))
        .arg(Arg::new("verbose").short('v').long("verbose").action(ArgAction::Count))
        .arg(Arg::new("quiet").short('q').long("quiet").action(ArgAction::Count))
        .arg(Arg::new("exampleconfig").long("exampleconfig").action(ArgAction::SetTrue))
        .get_matches();

    if matches.get_flag("exampleconfig") {
        println!(
            "# Reticulum Distributed Identity Resolver uses the Reticulum daemon configuration."
        );
    }
}
