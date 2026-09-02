// Kept separate from `new.rs` so that module stays below the repository's
// 500-line limit. `include!`d into the same module, so this is a file
// boundary and not a privacy one.

/// `Link.ESTABLISHMENT_TIMEOUT_PER_HOP` (`Reticulum.DEFAULT_PER_HOP_TIMEOUT`):
/// how long each hop is given for a link request to come back proven.
pub const ESTABLISHMENT_TIMEOUT_PER_HOP: Duration = Duration::from_secs(6);

/// What a link is given before the transport has sized it from the path:
/// the first hop plus one more, as for an unknown or single-hop path.
const DEFAULT_ESTABLISHMENT_TIMEOUT: Duration = Duration::from_secs(12);

impl Link {
    /// Bounds how long this link may stay unestablished. `RNS.Link.__init__`
    /// sizes it as the first hop's timeout plus
    /// [`ESTABLISHMENT_TIMEOUT_PER_HOP`] per hop to the destination, and the
    /// transport does the same when it creates an outbound link.
    pub fn set_establishment_timeout(&mut self, timeout: Duration) {
        self.establishment_timeout = timeout;
    }

    /// Whether a link that is still `Pending` or in `Handshake` has outlived
    /// its establishment timeout — `RNS.Link`'s watchdog closes it then with
    /// `teardown_reason = TIMEOUT`, and a non-transport instance expires the
    /// path it was made on. Measured from the link's creation, not from the
    /// latest repeated request.
    pub fn establishment_timed_out(&self, now: Instant) -> bool {
        matches!(self.status, LinkStatus::Pending | LinkStatus::Handshake)
            && now.duration_since(self.created_at) >= self.establishment_timeout
    }

    #[cfg(test)]
    pub(crate) fn set_created_at_for_test(&mut self, created_at: Instant) {
        self.created_at = created_at;
    }
}
