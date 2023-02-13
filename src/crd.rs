use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(CustomResource, Serialize, Deserialize, Debug, PartialEq, Clone, JsonSchema)]
#[kube(
    group = "vitalbh.io",
    version = "v1",
    kind = "PushOperator",
    plural = "pushoperators",
    derive = "PartialEq",
    namespaced
)]
pub struct PushOperatorSpec {
    pub filter: &str,
}