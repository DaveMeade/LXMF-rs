#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[non_exhaustive]
pub struct DeliveryStampPolicyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_cost: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flexibility: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforce: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Default)]
#[non_exhaustive]
pub struct DeliveryStampPolicyState {
    #[serde(default)]
    pub target_cost: Option<u32>,
    #[serde(default)]
    pub flexibility: Option<u32>,
    #[serde(default)]
    pub enforce: bool,
}

impl DeliveryStampPolicyState {
    fn from_policy(policy: &JsonValue) -> Self {
        Self {
            target_cost: policy
                .get("target_cost")
                .and_then(JsonValue::as_u64)
                .and_then(|value| u32::try_from(value).ok()),
            flexibility: policy
                .get("flexibility")
                .and_then(JsonValue::as_u64)
                .and_then(|value| u32::try_from(value).ok()),
            enforce: policy.get("enforce").and_then(JsonValue::as_bool).unwrap_or(false),
        }
    }
}

#[derive(Clone, Debug, Serialize, PartialEq)]
#[non_exhaustive]
pub struct DeliveryStampPolicyResult {
    #[serde(default)]
    pub stamp_policy: JsonValue,
    #[serde(default)]
    pub policy_state: DeliveryStampPolicyState,
}

#[derive(Deserialize)]
struct RawDeliveryStampPolicyResult {
    #[serde(default)]
    stamp_policy: JsonValue,
}

impl<'de> Deserialize<'de> for DeliveryStampPolicyResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = RawDeliveryStampPolicyResult::deserialize(deserializer)?;
        let policy_state = DeliveryStampPolicyState::from_policy(&raw.stamp_policy);
        Ok(Self {
            stamp_policy: raw.stamp_policy,
            policy_state,
        })
    }
}
