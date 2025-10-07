use dbt_serde_yaml::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;

use crate::schemas::dbt_column::ColumnProperties;
use crate::schemas::project::AnalysesConfig;
use crate::schemas::properties::GetConfig;

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalysesProperties {
    pub name: String,
    pub description: Option<String>,
    pub config: Option<AnalysesConfig>,
    pub columns: Option<Vec<ColumnProperties>>, // this column is much more permissive than whats actually allowed on the analysis
}

impl AnalysesProperties {
    pub fn empty(name: String) -> Self {
        Self {
            name,
            description: None,
            config: None,
            columns: None,
        }
    }
}

impl GetConfig<AnalysesConfig> for AnalysesProperties {
    fn get_config(&self) -> Option<&AnalysesConfig> {
        self.config.as_ref()
    }
}
