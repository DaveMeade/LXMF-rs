use clap::{Arg, ArgAction, Command};

const EXAMPLE_CONFIG: &str = "# This is an example package manager configuration file.";

fn main() {
    let matches = Command::new("rnpkg")
        .about("Reticulum Meta Package Manager")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(Arg::new("config").long("config").value_name("PATH"))
        .arg(Arg::new("verbose").short('v').long("verbose").action(ArgAction::Count))
        .arg(Arg::new("quiet").short('q').long("quiet").action(ArgAction::Count))
        .arg(Arg::new("exampleconfig").long("exampleconfig").action(ArgAction::SetTrue))
        .get_matches();

    if matches.get_flag("exampleconfig") {
        println!("{EXAMPLE_CONFIG}");
    }
}
