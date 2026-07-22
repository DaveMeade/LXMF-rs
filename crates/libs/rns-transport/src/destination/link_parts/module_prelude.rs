use std::{
    cmp::min,
    collections::{HashMap, VecDeque},
    panic::{catch_unwind, AssertUnwindSafe},
    time::{Duration, Instant},
};

use ed25519_dalek::{Signature, SigningKey, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH};

use rand_core::OsRng;

use sha2::Digest;

use x25519_dalek::StaticSecret;

use crate::{
    buffer::OutputBuffer,
    channel::{
        ChannelError, Envelope as ChannelEnvelope, Handler as ChannelHandler, HandlerId,
        MessageState as ChannelMessageState,
    },
    crypt::fernet::{CachedFernet, PlainText, Token},
    error::RnsError,
    hash::{AddressHash, Hash, ADDRESS_HASH_SIZE, HASH_SIZE},
    identity::{DecryptIdentity, DerivedKey, EncryptIdentity, Identity, PrivateIdentity},
    packet::{
        DestinationType, Header, Packet, PacketContext, PacketDataBuffer, PacketType, PACKET_MDU,
    },
};

use super::DestinationDesc;

const LINK_MTU_SIZE: usize = 3;

const LINK_MTU_MASK: u32 = 0x1F_FFFF;

const LINK_MODE_MASK: u32 = 0xE0_0000;

const RETICULUM_COMPAT_MTU: u32 = (PACKET_MDU + 2 + 1 + ADDRESS_HASH_SIZE * 2 + 1) as u32;

/// The link MTU value real clients advertise on an outbound LinkRequest —
/// confirmed by decoding a live wire capture of a known-good client's own
/// successful request, byte-for-byte: 8192 exactly. Distinct from
/// `RETICULUM_COMPAT_MTU` above, which is a ceiling for clamping an
/// *incoming* value, not a value meant to be sent outbound.
const LINK_ADVERTISED_MTU: u32 = 8192;

/// How many outbound attempts to make at the current `LinkMode` before
/// falling back to the other one — see `LinkMode`'s own doc comment for
/// why a fallback loop, rather than a single request, is the only
/// available strategy here.
const MODE_FALLBACK_ATTEMPTS: u8 = 2;

/// Reticulum Link's cipher-mode selector — packed into the top 3 bits of
/// the outbound LinkRequest's 3-byte MTU-signalling suffix. Previously
/// unrepresented in this crate at all: `Link::request()` never wrote the
/// signalling suffix, so every outbound request signalled mode `0` by
/// accident of the field being absent, not by choice.
///
/// Both values below were confirmed empirically against a real
/// destination's own log output: mode `0` ("Incoming link request with
/// mode AES_128_CBC", then "Requested link mode AES_128_CBC not enabled"
/// — that destination has it disabled) and mode `1` ("AES_256_CBC", which
/// the same destination accepted and proved). A live wire capture of a
/// known-good reference client's own successful LinkRequest confirmed it
/// always sends mode `1` too, matching `LinkMode::DEFAULT` below.
/// `LINK_MODE_MASK` reserves 3 bits (up to 8 possible values) but only
/// these two have ever been observed — treat any other value as unknown,
/// not as "a mode this crate also supports."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkMode {
    Aes128Cbc = 0,
    Aes256Cbc = 1,
}

impl LinkMode {
    /// The mode a known-good reference client actually requests by
    /// default — this crate's own new outbound default too.
    pub const DEFAULT: LinkMode = LinkMode::Aes256Cbc;

    /// The other of the two known modes — there being only two, "fall
    /// back" and "fall forward" are the same operation.
    fn fallback(self) -> LinkMode {
        match self {
            LinkMode::Aes256Cbc => LinkMode::Aes128Cbc,
            LinkMode::Aes128Cbc => LinkMode::Aes256Cbc,
        }
    }

    fn mode_bits(self) -> u32 {
        self as u32
    }
}

impl Default for LinkMode {
    fn default() -> Self {
        LinkMode::DEFAULT
    }
}

const KEEPALIVE_MAX_RTT: f32 = 1.75;

const KEEPALIVE_TIMEOUT_FACTOR: f32 = 4.0;

const STALE_GRACE_SECS: f32 = 5.0;

const KEEPALIVE_MAX_SECS: f32 = 360.0;

const KEEPALIVE_MIN_SECS: f32 = 5.0;

const STALE_FACTOR: f32 = 2.0;

const CHANNEL_RX_WINDOW_MAX: u16 = 48;

const CHANNEL_WINDOW_INIT: u8 = 2;

const CHANNEL_WINDOW_MIN: u8 = 2;

const CHANNEL_WINDOW_MIN_LIMIT_MEDIUM: u8 = 5;

const CHANNEL_WINDOW_MIN_LIMIT_FAST: u8 = 16;

const CHANNEL_WINDOW_MAX_SLOW: u8 = 5;

const CHANNEL_WINDOW_MAX_MEDIUM: u8 = 12;

const CHANNEL_WINDOW_MAX_FAST: u8 = 48;

const CHANNEL_FAST_RATE_THRESHOLD: u8 = 10;

const CHANNEL_RTT_FAST_SECS: f32 = 0.18;

const CHANNEL_RTT_MEDIUM_SECS: f32 = 0.75;

const CHANNEL_RTT_SLOW_SECS: f32 = 1.45;

const CHANNEL_WINDOW_FLEXIBILITY: u8 = 4;

#[allow(dead_code)]
const CHANNEL_MAX_TRIES: u8 = 5;

#[derive(Debug, Clone)]
struct PendingChannelPacket {
    sequence: u16,
    #[allow(dead_code)]
    packet: Packet,
    #[allow(dead_code)]
    tries: u8,
    #[allow(dead_code)]
    next_retry_at: Instant,
}

struct RegisteredChannelHandler {
    id: HandlerId,
    handler: ChannelHandler,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum LinkStatus {
    Pending = 0x00,
    Handshake = 0x01,
    Active = 0x02,
    Stale = 0x03,
    Closed = 0x04,
}

impl LinkStatus {
    pub fn not_yet_active(&self) -> bool {
        *self == LinkStatus::Pending || *self == LinkStatus::Handshake
    }

    fn can_exchange_data(self) -> bool {
        matches!(self, Self::Active)
    }

    #[allow(dead_code)]
    fn can_retry_channel_messages(self) -> bool {
        matches!(self, Self::Active | Self::Stale)
    }

    fn can_send_teardown(self) -> bool {
        matches!(self, Self::Active | Self::Stale)
    }
}

pub type LinkId = AddressHash;
