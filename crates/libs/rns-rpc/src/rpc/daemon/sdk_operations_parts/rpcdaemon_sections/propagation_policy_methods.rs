fn propagation_policy_method(method: &str) -> bool {
    matches!(
        method,
        "set_authentication"
            | "requires_authentication"
            | "allow"
            | "disallow"
            | "ignore_destination"
            | "unignore_destination"
            | "prioritise"
            | "unprioritise"
    )
}
