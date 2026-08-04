// The negotiated link MTU, split out of `new.rs` to stay inside the
// repository's 500-line module budget. `include!`d into the same module.

impl Link {
    /// The MTU negotiated for this link — the smaller of what the two peers
    /// signalled, or [`LEGACY_RETICULUM_MTU`] if the peer signalled nothing.
    ///
    /// **This, not the local interface MTU, is what may size anything put on
    /// the wire.** The interface bounds what this node can physically carry;
    /// the negotiated value bounds what the whole path can carry, and a
    /// resource fragment sized from the former will be silently dropped by
    /// any hop that cannot take it.
    pub fn link_mtu(&self) -> usize {
        link_signalled_mtu(self.signalling)
    }
}
