use std::net::IpAddr;

use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::config::AppConfig;

const OPERATOR_TOKEN_HEADER: &str = "x-operator-api-token";
const OPERATOR_TOKEN_HEADER_ALIAS: &str = "x-operator-token";

#[derive(Debug, Clone)]
pub struct ApiSecurityConfig {
    api_host: IpAddr,
    api_port: u16,
    allow_lan_dashboard: bool,
    operator_api_token: Option<String>,
    allowed_origins: Vec<String>,
}

impl ApiSecurityConfig {
    pub fn from_app_config(config: &AppConfig) -> Self {
        let allow_lan_dashboard = env_bool("ALLOW_LAN_DASHBOARD", false);
        let operator_api_token = std::env::var("OPERATOR_TOKEN")
            .or_else(|_| std::env::var("OPERATOR_API_TOKEN"))
            .ok()
            .filter(|value| !value.trim().is_empty());
        let allowed_origin = std::env::var("ALLOWED_DASHBOARD_ORIGIN")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| format!("http://127.0.0.1:{}", config.api_port));
        let mut allowed_origins = vec![
            allowed_origin,
            format!("http://127.0.0.1:{}", config.api_port),
            format!("http://localhost:{}", config.api_port),
        ];
        allowed_origins.sort();
        allowed_origins.dedup();

        Self {
            api_host: config.api_host,
            api_port: config.api_port,
            allow_lan_dashboard,
            operator_api_token,
            allowed_origins,
        }
    }

    pub fn cors_layer(&self) -> CorsLayer {
        let allowed_origins = self.allowed_origins.clone();
        CorsLayer::new()
            .allow_methods([Method::GET, Method::POST])
            .allow_headers([
                header::CONTENT_TYPE,
                header::AUTHORIZATION,
                axum::http::HeaderName::from_static(OPERATOR_TOKEN_HEADER),
                axum::http::HeaderName::from_static(OPERATOR_TOKEN_HEADER_ALIAS),
            ])
            .allow_origin(AllowOrigin::predicate(move |origin, _request_head| {
                allowed_origins.iter().any(|allowed| {
                    HeaderValue::from_str(allowed)
                        .map(|allowed| origin == allowed)
                        .unwrap_or(false)
                })
            }))
    }
}

pub async fn guard_operator_post_requests(
    State(security): State<ApiSecurityConfig>,
    request: Request,
    next: Next,
) -> Response {
    if request.method() != Method::POST {
        if sensitive_get_requires_token(&security, &request) {
            let status = if security.operator_api_token.is_some() {
                StatusCode::UNAUTHORIZED
            } else {
                StatusCode::FORBIDDEN
            };
            return (
                status,
                Json(serde_json::json!({
                    "ok": false,
                    "readOnly": true,
                    "runtimeModified": false,
                    "reason": if security.operator_api_token.is_some() {
                        "operator_token_required"
                    } else {
                        "operator_token_missing_for_non_loopback_api"
                    },
                    "message": "Non-loopback API access requires X-Operator-Token or Authorization: Bearer token."
                })),
            )
                .into_response();
        }
        return next.run(request).await;
    }

    if post_allowed(&security, request.headers()) {
        return next.run(request).await;
    }

    (
        StatusCode::FORBIDDEN,
        Json(serde_json::json!({
            "ok": false,
            "readOnly": true,
            "runtimeModified": false,
            "reason": "operator_api_guard_rejected",
            "message": "Protected POST requires local dashboard access or an operator token when LAN dashboard access is enabled."
        })),
    )
        .into_response()
}

fn sensitive_get_requires_token(security: &ApiSecurityConfig, request: &Request) -> bool {
    if request.method() != Method::GET || security.api_host.is_loopback() {
        return false;
    }
    let path = request.uri().path();
    if !(path.starts_with("/api/") || path.starts_with("/ws/")) {
        return false;
    }
    if token_matches(security, request.headers()) {
        return false;
    }
    true
}

fn post_allowed(security: &ApiSecurityConfig, headers: &HeaderMap) -> bool {
    let origin_allowed = origin_is_allowed(security, headers);
    if !origin_allowed {
        return false;
    }

    if security.api_host.is_loopback() && host_is_loopback(headers, security.api_port) {
        return true;
    }

    (security.allow_lan_dashboard || security.operator_api_token.is_some())
        && token_matches(security, headers)
}

fn origin_is_allowed(security: &ApiSecurityConfig, headers: &HeaderMap) -> bool {
    let Some(origin) = headers.get(header::ORIGIN) else {
        return true;
    };
    security
        .allowed_origins
        .iter()
        .any(|allowed| origin.as_bytes() == allowed.as_bytes())
}

fn token_matches(security: &ApiSecurityConfig, headers: &HeaderMap) -> bool {
    let Some(expected) = security.operator_api_token.as_deref() else {
        return false;
    };
    let header_token = headers
        .get(OPERATOR_TOKEN_HEADER)
        .or_else(|| headers.get(OPERATOR_TOKEN_HEADER_ALIAS))
        .and_then(|value| value.to_str().ok());
    if header_token == Some(expected) {
        return true;
    }
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        == Some(expected)
}

fn host_is_loopback(headers: &HeaderMap, api_port: u16) -> bool {
    let Some(host) = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    let host_without_port = host
        .strip_prefix('[')
        .and_then(|value| value.split(']').next())
        .or_else(|| host.split(':').next())
        .unwrap_or(host);
    matches!(host_without_port, "127.0.0.1" | "localhost" | "::1")
        || host == format!("127.0.0.1:{api_port}")
        || host == format!("localhost:{api_port}")
}

fn env_bool(key: &str, default: bool) -> bool {
    std::env::var(key)
        .ok()
        .map(|value| value.eq_ignore_ascii_case("true"))
        .unwrap_or(default)
}
