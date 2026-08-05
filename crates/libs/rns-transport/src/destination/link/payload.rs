/// A decrypted link payload.
///
/// The buffer is heap-allocated rather than a fixed `[u8; PACKET_MDU]`
/// because a link's MTU is negotiated, not constant: once a peer signals a
/// larger one, a single data packet can legitimately carry far more than
/// 464 bytes. The fixed array silently `min()`-truncated anything larger —
/// so a correctly-sized Response from a peer that had agreed a bigger MTU
/// arrived as its first 464 bytes and failed to parse, with no error
/// anywhere. That is the failure #498 hit when it advertised 8192.
///
/// It also removes a latent panic: `new_from_vec` set `len` from the input
/// while copying only what fitted, so `as_slice()` on an oversized payload
/// indexed past the end of the array.
#[derive(Clone)]
pub struct LinkPayload {
    buffer: Vec<u8>,
    len: usize,
    context: PacketContext,
    request_id: Option<[u8; ADDRESS_HASH_SIZE]>,
}

impl LinkPayload {
    pub fn new() -> Self {
        Self { buffer: Vec::new(), len: 0, context: PacketContext::None, request_id: None }
    }

    pub fn new_from_slice(data: &[u8]) -> Self {
        Self::new_from_slice_with_context(data, PacketContext::None)
    }

    pub fn new_from_slice_with_context(data: &[u8], context: PacketContext) -> Self {
        Self { buffer: data.to_vec(), len: data.len(), context, request_id: None }
    }

    pub fn new_from_slice_with_context_and_request_id(
        data: &[u8],
        context: PacketContext,
        request_id: Option<[u8; ADDRESS_HASH_SIZE]>,
    ) -> Self {
        let mut payload = Self::new_from_slice_with_context(data, context);
        payload.request_id = request_id;
        payload
    }

    pub fn new_from_vec(data: &[u8]) -> Self {
        Self { buffer: data.to_vec(), len: data.len(), context: PacketContext::None, request_id: None }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn context(&self) -> PacketContext {
        self.context
    }

    pub fn request_id(&self) -> Option<[u8; ADDRESS_HASH_SIZE]> {
        self.request_id
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buffer[..self.len]
    }
}

impl Default for LinkPayload {
    fn default() -> Self {
        Self::new()
    }
}
