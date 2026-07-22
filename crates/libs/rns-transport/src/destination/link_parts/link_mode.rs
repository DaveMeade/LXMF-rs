/// Reticulum Link's cipher-mode selector — packed into the top 3 bits of
/// the outbound LinkRequest's 3-byte MTU-signalling suffix (see
/// `Link::request`'s own doc comment for the signalling story). Previously
/// unrepresented in this crate at all: `Link::request()` never wrote the
/// signalling suffix, so every outbound request signalled mode `0` by
/// accident of the field being absent, not by choice.
///
/// Both values below were confirmed empirically against a real
/// destination's own log output: mode `0` ("Incoming link request with
/// mode AES_128_CBC", then "Requested link mode AES_128_CBC not enabled" —
/// that destination has it disabled) and mode `1` ("AES_256_CBC", which the
/// same destination accepted and proved). `LINK_MODE_MASK` reserves 3 bits
/// (up to 8 possible values) but only these two have ever been observed —
/// treat any other value as unknown, not as "a mode this crate also
/// supports."
///
/// Critically, this crate can only ever *use* one of these two per build,
/// never choose between them at runtime: `crypt::fernet::AesAlgo` is a
/// compile-time type alias (`aes::Aes128` behind the `fernet-aes128`
/// feature, `aes::Aes256` otherwise), so `CachedFernet`'s actual AES key
/// size is fixed for the whole binary. `DEFAULT` below tracks that same
/// feature flag rather than being a fixed constant — advertising (or
/// falling back to) the *other* mode would let a peer accept and prove a
/// Link whose two sides then try to encrypt/decrypt with different AES key
/// sizes, which can never actually exchange data. There is therefore no
/// fallback between modes: with only one cipher compiled in, there is
/// nothing else to fall back to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkMode {
    Aes128Cbc = 0,
    Aes256Cbc = 1,
}

impl LinkMode {
    /// The one mode this build can actually speak, per the `fernet-aes128`
    /// feature — see this enum's own doc comment for why it must track
    /// that flag rather than being fixed.
    #[cfg(feature = "fernet-aes128")]
    pub const DEFAULT: LinkMode = LinkMode::Aes128Cbc;
    #[cfg(not(feature = "fernet-aes128"))]
    pub const DEFAULT: LinkMode = LinkMode::Aes256Cbc;

    fn mode_bits(self) -> u32 {
        self as u32
    }
}

impl Default for LinkMode {
    fn default() -> Self {
        LinkMode::DEFAULT
    }
}

#[cfg(test)]
mod link_mode_tests {
    use super::*;

    #[test]
    fn default_matches_the_compiled_in_cipher() {
        // Whichever feature is active for *this* test run, DEFAULT must
        // never claim the mode the build can't actually encrypt with.
        #[cfg(feature = "fernet-aes128")]
        assert_eq!(LinkMode::DEFAULT, LinkMode::Aes128Cbc);
        #[cfg(not(feature = "fernet-aes128"))]
        assert_eq!(LinkMode::DEFAULT, LinkMode::Aes256Cbc);
    }

    #[test]
    fn mode_bits_round_trip_the_wire_values_confirmed_live() {
        assert_eq!(LinkMode::Aes128Cbc.mode_bits(), 0);
        assert_eq!(LinkMode::Aes256Cbc.mode_bits(), 1);
    }
}
