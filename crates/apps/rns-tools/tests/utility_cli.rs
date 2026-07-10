use std::process::Command;

#[test]
fn rnpkg_exposes_python_compatible_example_config() {
    let output = Command::new(env!("CARGO_BIN_EXE_rnpkg"))
        .arg("--exampleconfig")
        .output()
        .expect("run rnpkg");
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("UTF-8 output").trim(),
        "# This is an example package manager configuration file."
    );
}

#[test]
fn rnir_and_rnpkg_accept_reference_global_options() {
    for binary in [env!("CARGO_BIN_EXE_rnir"), env!("CARGO_BIN_EXE_rnpkg")] {
        let status = Command::new(binary)
            .args(["--config", "/tmp/rns-config", "-vv", "-q"])
            .status()
            .expect("run utility");
        assert!(status.success());
    }
}
