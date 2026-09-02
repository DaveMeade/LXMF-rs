// The reply to `LoraConfig::rom_read_frame()`, read back. `include!`d into
// the lora module beside the frame builders; its own file so each stays
// within the repository's module-size policy.

/// Byte offsets into an RNode's EEPROM image — `rnodeconf`'s `ROM.ADDR_CONF_*`.
mod rom {
    pub const ADDR_CONF_SF: usize = 0x9C;
    pub const ADDR_CONF_CR: usize = 0x9D;
    pub const ADDR_CONF_TXP: usize = 0x9E;
    /// Four bytes, big-endian.
    pub const ADDR_CONF_BW: usize = 0x9F;
    /// Four bytes, big-endian.
    pub const ADDR_CONF_FREQ: usize = 0xA3;
    /// Holds [`CONF_OK_BYTE`] when, and only when, a configuration is stored.
    pub const ADDR_CONF_OK: usize = 0xA7;
    /// `rnodeconf`'s `ROM.CONF_OK_BYTE`.
    pub const CONF_OK_BYTE: u8 = 0x73;
}

/// The shortest EEPROM image that can say whether a configuration is stored.
/// Anything shorter is a truncated read, not a device without one.
const STORED_CONFIG_IMAGE_LEN: usize = rom::ADDR_CONF_OK + 1;

/// The radio settings an RNode holds in its EEPROM: what `rnodeconf --tnc`
/// saved with `CMD_CONF_SAVE`, and what the device starts up on.
///
/// Deliberately not a [`LoraConfig`]: this is what the device reported, not
/// what a caller intends to apply, and the two are different facts until a
/// person has confirmed the first. The payload limits and airtime limits a
/// `LoraConfig` carries are not stored on the device at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StoredRadioConfig {
    pub frequency_hz: u32,
    pub bandwidth_hz: u32,
    pub spreading_factor: u8,
    pub coding_rate: u8,
    pub tx_power_dbm: u8,
}

impl LoraConfig {
    /// Reads the stored configuration out of a `CMD_ROM_READ` reply — the
    /// EEPROM image — the way `rnodeconf` reads it: only when
    /// `ADDR_CONF_OK` carries `CONF_OK_BYTE`, with the two multi-byte
    /// fields big-endian.
    ///
    /// `None` means the device holds no configuration, or the image is too
    /// short to tell. It is never a default: a default here is how a radio
    /// ends up transmitting on a frequency nobody chose.
    #[must_use]
    pub fn parse_stored_config(eeprom: &[u8]) -> Option<StoredRadioConfig> {
        if eeprom.len() < STORED_CONFIG_IMAGE_LEN || eeprom[rom::ADDR_CONF_OK] != rom::CONF_OK_BYTE
        {
            return None;
        }
        Some(StoredRadioConfig {
            frequency_hz: be_u32(eeprom, rom::ADDR_CONF_FREQ),
            bandwidth_hz: be_u32(eeprom, rom::ADDR_CONF_BW),
            spreading_factor: eeprom[rom::ADDR_CONF_SF],
            coding_rate: eeprom[rom::ADDR_CONF_CR],
            tx_power_dbm: eeprom[rom::ADDR_CONF_TXP],
        })
    }
}

/// Big-endian `u32` at `offset`; the caller has bounds-checked against
/// [`STORED_CONFIG_IMAGE_LEN`], which covers every field read here.
fn be_u32(bytes: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3]])
}

#[cfg(test)]
mod stored_config_tests {
    use super::*;

    /// Real values from a Heltec LoRa32 v4 on firmware 1.86, as
    /// `rnodeconf -i` reports them — a byte-order or offset mistake then
    /// reads as an obviously wrong number rather than a plausible one.
    fn device_image() -> Vec<u8> {
        let mut eeprom = vec![0u8; STORED_CONFIG_IMAGE_LEN];
        eeprom[rom::ADDR_CONF_SF] = 11;
        eeprom[rom::ADDR_CONF_CR] = 5;
        eeprom[rom::ADDR_CONF_TXP] = 22;
        eeprom[rom::ADDR_CONF_BW..rom::ADDR_CONF_BW + 4]
            .copy_from_slice(&250_000u32.to_be_bytes());
        eeprom[rom::ADDR_CONF_FREQ..rom::ADDR_CONF_FREQ + 4]
            .copy_from_slice(&917_375_000u32.to_be_bytes());
        eeprom[rom::ADDR_CONF_OK] = rom::CONF_OK_BYTE;
        eeprom
    }

    #[test]
    fn reads_the_configuration_a_device_is_holding() {
        let parsed = LoraConfig::parse_stored_config(&device_image()).expect("CONF_OK is set");

        assert_eq!(
            parsed,
            StoredRadioConfig {
                frequency_hz: 917_375_000,
                bandwidth_hz: 250_000,
                spreading_factor: 11,
                coding_rate: 5,
                tx_power_dbm: 22,
            }
        );
    }

    /// Only the exact sentinel counts, so a half-written EEPROM cannot pass
    /// for a configuration.
    #[test]
    fn a_device_holding_no_configuration_reads_as_none() {
        let mut eeprom = device_image();
        eeprom[rom::ADDR_CONF_OK] = 0x00;
        assert!(LoraConfig::parse_stored_config(&eeprom).is_none());

        eeprom[rom::ADDR_CONF_OK] = 0x72;
        assert!(LoraConfig::parse_stored_config(&eeprom).is_none());
    }

    /// A truncated read and an unconfigured device are different facts that
    /// want the same answer here; a future change that starts trusting short
    /// images has to do it deliberately.
    #[test]
    fn a_truncated_image_is_not_mistaken_for_a_configuration() {
        let full = device_image();
        for len in [0, rom::ADDR_CONF_SF, rom::ADDR_CONF_FREQ, STORED_CONFIG_IMAGE_LEN - 1] {
            assert!(
                LoraConfig::parse_stored_config(&full[..len]).is_none(),
                "an image of {len} bytes cannot answer the question"
            );
        }
        assert!(LoraConfig::parse_stored_config(&full[..STORED_CONFIG_IMAGE_LEN]).is_some());
    }

    /// Read little-endian, 917.375 MHz becomes 3.4 GHz.
    #[test]
    fn multi_byte_fields_are_big_endian() {
        let parsed = LoraConfig::parse_stored_config(&device_image()).expect("configured");

        assert_eq!(parsed.frequency_hz, 917_375_000);
        assert_ne!(parsed.frequency_hz, u32::from_le_bytes(917_375_000u32.to_be_bytes()));
    }
}
