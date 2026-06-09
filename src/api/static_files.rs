use axum::{
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
};

pub async fn root(headers: HeaderMap) -> Html<&'static str> {
    log_spa_access("/", &headers);
    dashboard().await
}

pub async fn spa(headers: HeaderMap) -> Html<&'static str> {
    log_spa_access("spa", &headers);
    dashboard().await
}

pub async fn dashboard() -> Html<&'static str> {
    Html(include_str!("../../web/index.html"))
}

pub async fn app_js() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/javascript; charset=utf-8"),
        )],
        include_str!("../../web/app.js"),
    )
        .into_response()
}

pub async fn styles_css() -> Response {
    (
        [(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/css; charset=utf-8"),
        )],
        include_str!("../../web/styles.css"),
    )
        .into_response()
}

pub fn not_found(message: &str) -> Response {
    (StatusCode::NOT_FOUND, message.to_string()).into_response()
}

fn log_spa_access(route: &str, headers: &HeaderMap) {
    let user_agent = headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(sanitize_user_agent)
        .unwrap_or_else(|| "unknown".to_string());
    tracing::info!(
        target: "web_access",
        route,
        user_agent = %user_agent,
        "spa_route_requested"
    );
}

fn sanitize_user_agent(value: &str) -> String {
    value
        .chars()
        .filter(|character| !character.is_control())
        .take(180)
        .collect()
}
