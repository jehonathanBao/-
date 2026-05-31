use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiSafetyContract {
    pub read_only: bool,
    pub runtime_modified: bool,
    pub analysis_only: bool,
    pub execution_enabled: bool,
    pub manual_review_required: bool,
    pub runtime_weight_modified: bool,
    pub config_modified: bool,
}

impl ApiSafetyContract {
    pub const fn analysis_only() -> Self {
        Self {
            read_only: true,
            runtime_modified: false,
            analysis_only: true,
            execution_enabled: false,
            manual_review_required: false,
            runtime_weight_modified: false,
            config_modified: false,
        }
    }

    pub const fn manual_review() -> Self {
        Self {
            manual_review_required: true,
            ..Self::analysis_only()
        }
    }
}
