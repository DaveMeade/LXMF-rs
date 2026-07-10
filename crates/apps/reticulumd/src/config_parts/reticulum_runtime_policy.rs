#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReticulumRuntimePolicy {
    pub link_mtu_discovery: bool,
    pub remote_management_enabled: bool,
    pub respond_to_probes: bool,
    pub use_implicit_proof: bool,
    pub discover_interfaces: bool,
    pub required_discovery_value: Option<u32>,
    pub publish_blackhole: bool,
    pub blackhole_sources: Vec<String>,
    pub interface_discovery_sources: Vec<String>,
    pub max_autoconnected_interfaces: u32,
}

impl Default for ReticulumRuntimePolicy {
    fn default() -> Self {
        Self {
            link_mtu_discovery: true,
            remote_management_enabled: false,
            respond_to_probes: false,
            use_implicit_proof: true,
            discover_interfaces: false,
            required_discovery_value: None,
            publish_blackhole: false,
            blackhole_sources: Vec::new(),
            interface_discovery_sources: Vec::new(),
            max_autoconnected_interfaces: 0,
        }
    }
}

impl ReticulumRuntimePolicy {
    pub fn from_toml(input: &str) -> Result<Self, toml::de::Error> {
        #[derive(serde::Deserialize)]
        struct Document {
            #[serde(default)]
            reticulum: Option<ReticulumConfigRaw>,
        }

        let document: Document = toml::from_str(input)?;
        document
            .reticulum
            .as_ref()
            .map(ReticulumConfigRaw::runtime_policy)
            .transpose()
            .map_err(|error| <toml::de::Error as serde::de::Error>::custom(error))
            .map(Option::unwrap_or_default)
    }
}

impl ReticulumConfigRaw {
    fn runtime_policy(&self) -> Result<ReticulumRuntimePolicy, String> {
        Ok(ReticulumRuntimePolicy {
            link_mtu_discovery: self.link_mtu_discovery.unwrap_or(true),
            remote_management_enabled: self.enable_remote_management.unwrap_or(false),
            respond_to_probes: self.respond_to_probes.unwrap_or(false),
            use_implicit_proof: self.use_implicit_proof.unwrap_or(true),
            discover_interfaces: self.discover_interfaces.unwrap_or(false),
            required_discovery_value: self
                .required_discovery_value
                .filter(|value| *value > 0)
                .map(|value| value as u32),
            publish_blackhole: self.publish_blackhole.unwrap_or(false),
            blackhole_sources: validate_identity_hash_list(
                "blackhole source",
                &self.blackhole_sources,
            )?,
            interface_discovery_sources: validate_identity_hash_list(
                "interface discovery source",
                &self.interface_discovery_sources,
            )?,
            max_autoconnected_interfaces: self
                .autoconnect_discovered_interfaces
                .filter(|value| *value > 0)
                .map(|value| value as u32)
                .unwrap_or(0),
        })
    }
}

fn validate_identity_hash_list(label: &str, values: &[String]) -> Result<Vec<String>, String> {
    let mut validated = Vec::new();
    for value in values {
        let normalized = value.trim().to_ascii_lowercase();
        if normalized.len() != 32 || hex::decode(&normalized).is_err() {
            return Err(format!(
                "{label} {value} is invalid, must be 32 hexadecimal characters (16 bytes)"
            ));
        }
        if !validated.contains(&normalized) {
            validated.push(normalized);
        }
    }
    Ok(validated)
}
