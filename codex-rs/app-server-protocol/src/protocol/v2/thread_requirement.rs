use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use ts_rs::TS;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadRequirementReadParams {
    pub thread_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadRequirementReadResponse {
    pub requirement: ThreadRequirement,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadDecisionListParams {
    pub thread_id: String,
    #[ts(optional = nullable)]
    pub status: Option<ThreadDecisionStatus>,
    #[ts(optional = nullable)]
    pub urgency: Option<ThreadDecisionUrgency>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadDecisionListResponse {
    pub data: Vec<ThreadDecision>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadDecisionResolveParams {
    pub thread_id: String,
    pub decision_id: String,
    #[ts(optional = nullable)]
    pub selected_option_id: Option<String>,
    #[ts(optional = nullable)]
    pub resolution: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub defer: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadDecisionResolveResponse {
    pub decision: ThreadDecision,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadRequirement {
    pub thread_id: String,
    pub objective: Option<String>,
    pub status: ThreadRequirementStatus,
    pub summary: String,
    pub decisions: Vec<ThreadDecision>,
    #[ts(type = "number | null")]
    pub updated_at: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum ThreadRequirementStatus {
    NotStarted,
    Running,
    WaitingOnDecision,
    Complete,
    Failed,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadDecision {
    pub id: String,
    pub thread_id: String,
    pub title: String,
    pub description: String,
    pub urgency: ThreadDecisionUrgency,
    pub status: ThreadDecisionStatus,
    pub options: Vec<ThreadDecisionOption>,
    pub recommendation: Option<String>,
    pub source_turn_id: Option<String>,
    #[ts(type = "number | null")]
    pub resolved_at: Option<i64>,
    pub resolution: Option<String>,
    pub selected_option_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum ThreadDecisionUrgency {
    Immediate,
    Deferred,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename_all = "camelCase", export_to = "v2/")]
pub enum ThreadDecisionStatus {
    Pending,
    Resolved,
    Deferred,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "v2/")]
pub struct ThreadDecisionOption {
    pub id: String,
    pub label: String,
    pub description: Option<String>,
}
