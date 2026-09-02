/// `LXMRouter.PROPAGATION_COST` is what a sender mines to when the relay's
/// announced cost is unknown. The daemon used the minimum a default relay
/// accepts instead (the target minus `PROPAGATION_COST_FLEX`), so a relay
/// configured with no flexibility rejected every stamp it produced.
#[test]
fn propagation_default_stamp_cost_is_the_python_target_not_the_minimum_accepted() {
    assert_eq!(crate::bridge::DEFAULT_PROPAGATION_STAMP_COST, 16);
    assert_eq!(
        crate::bridge::DEFAULT_PROPAGATION_STAMP_COST,
        lxmf::stamp::DEFAULT_PROPAGATION_STAMP_COST
    );
}
