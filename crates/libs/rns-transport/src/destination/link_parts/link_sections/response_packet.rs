// Kept separate from `new.rs` so that module stays below the repository's
// 500-line limit — the same reason `identify_packet_tests.rs` is its own
// file. `include!`d into the same module, so this is a file boundary and
// not a privacy one.

impl Link {
    /// Build a response packet (context = `Response`) carrying `data`.
    ///
    /// Real Reticulum answers a request with a single packet whenever the
    /// packed response fits the link MDU, and only falls back to a resource
    /// transfer when it does not (`RNS/Link.py`, `handle_request`):
    ///
    /// ```text
    /// packed_response = umsgpack.packb([request_id, response])
    /// if len(packed_response) <= self.mdu:
    ///     RNS.Packet(self, packed_response, RNS.Packet.DATA,
    ///                context = RNS.Packet.RESPONSE).send()
    /// else:
    ///     response_resource = RNS.Resource(packed_response, self, ...)
    /// ```
    ///
    /// `data` is that already-packed `[request_id, response]` envelope —
    /// the same bytes either branch carries, so a responder only chooses
    /// the mechanism, never the payload.
    ///
    /// The receive half has always been here: `handle_packet` decrypts
    /// `PacketContext::Response` and posts it as a `LinkEvent::Data`. Only
    /// the constructor was missing, so a responder built on this crate had
    /// to send every reply as a resource — several round trips and an
    /// advertisement for a payload that fits in one packet, which on a slow
    /// link costs far more than the bytes do.
    pub fn response_packet(&self, data: &[u8]) -> Result<Packet, RnsError> {
        self.packet_with_context(data, PacketContext::Response)
    }
}
