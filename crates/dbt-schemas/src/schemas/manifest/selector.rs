use std::collections::BTreeMap;

use dbt_serde_yaml::Value;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub struct DbtSelector {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub __definition__: Value,
    pub __other__: BTreeMap<String, Value>,
}
