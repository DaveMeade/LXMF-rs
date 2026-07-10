#[test]
fn parses_reticulum_panic_on_interface_error_policy() {
    let default_cfg = DaemonConfig::from_toml("").expect("parse empty config");
    assert!(!default_cfg.panic_on_interface_error);

    let cfg = DaemonConfig::from_toml(
        r#"
[reticulum]
panic_on_interface_error = true
"#,
    )
    .expect("parse panic_on_interface_error config");
    assert!(cfg.panic_on_interface_error);
}
