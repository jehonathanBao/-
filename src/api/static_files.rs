use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
};

pub async fn root() -> Html<&'static str> {
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
