use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, time::Duration};

#[derive(Debug, Serialize)]
pub struct LangflowRiskRequest {
    pub order_id: String,
    pub tenant_id: String,
    pub shop_id: String,
    pub features: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct LangflowRiskResponse {
    pub risk_score: f64,
    pub risk_level: String,
    pub reason: String,
    pub suggested_action: String,
    pub requires_manual_review: bool,
}

pub struct LangflowRiskClient {
    endpoint: Url,
    http: reqwest::Client,
}

impl LangflowRiskClient {
    pub fn new(endpoint: Url) -> anyhow::Result<Self> {
        validate_sidecar_url(&endpoint)?;
        Ok(Self {
            endpoint,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(3))
                .build()?,
        })
    }

    pub async fn score(&self, request: &LangflowRiskRequest) -> anyhow::Result<LangflowRiskResponse> {
        let response = self
            .http
            .post(self.endpoint.clone())
            .json(request)
            .send()
            .await?
            .error_for_status()?;
        let bytes = response.bytes().await?;
        anyhow::ensure!(bytes.len() <= 64 * 1024, "langflow_response_too_large");
        Ok(serde_json::from_slice(&bytes)?)
    }
}

fn validate_sidecar_url(url: &Url) -> anyhow::Result<()> {
    anyhow::ensure!(url.scheme() == "https", "sidecar_url_must_use_https");
    let host = url.host_str().unwrap_or_default();
    anyhow::ensure!(!host.eq_ignore_ascii_case("localhost"), "localhost_blocked");
    if let Ok(ip) = host.parse::<IpAddr>() {
        anyhow::ensure!(!ip.is_loopback(), "loopback_blocked");
        anyhow::ensure!(!ip.is_private(), "private_ip_blocked");
        anyhow::ensure!(!ip.is_link_local(), "link_local_blocked");
    }
    Ok(())
}
