#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct DeliveryTicketGenerateRequest {
    pub destination: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl_secs: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub struct DeliveryTicketGenerateResult {
    pub destination: String,
    #[serde(default)]
    pub ticket: Option<String>,
    #[serde(default)]
    pub expires_at: Option<i64>,
    pub ttl_secs: u64,
    #[serde(default)]
    pub included: bool,
    #[serde(default)]
    pub reason: Option<String>,
}
