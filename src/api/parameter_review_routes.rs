use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};

use crate::{
    app::AppState,
    calibration::parameter_recommendation_review_store::{
        ParameterRecommendationReviewStore, ReviewDecisionInput,
    },
};

pub async fn parameter_recommendations(State(state): State<AppState>) -> impl IntoResponse {
    let store = review_store(&state);
    match store.list_recommendations() {
        Ok(recommendations) => {
            Json(serde_json::json!({ "recommendations": recommendations })).into_response()
        }
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn latest_parameter_recommendations(State(state): State<AppState>) -> impl IntoResponse {
    let store = review_store(&state);
    match store.latest_recommendations() {
        Ok(recommendations) => {
            Json(serde_json::json!({ "recommendations": recommendations })).into_response()
        }
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn parameter_recommendation_by_id(
    State(state): State<AppState>,
    Path(recommendation_id): Path<String>,
) -> impl IntoResponse {
    let store = review_store(&state);
    match store.get_recommendation(&recommendation_id) {
        Ok(Some(recommendation)) => {
            Json(serde_json::json!({ "recommendation": recommendation })).into_response()
        }
        Ok(None) => json_error(
            StatusCode::NOT_FOUND,
            "recommendation not found".to_string(),
        ),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn parameter_reviews(State(state): State<AppState>) -> impl IntoResponse {
    let store = review_store(&state);
    match store.list_reviews() {
        Ok(reviews) => Json(serde_json::json!({ "reviews": reviews })).into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

pub async fn create_parameter_review(
    State(state): State<AppState>,
    Json(input): Json<ReviewDecisionInput>,
) -> impl IntoResponse {
    let store = review_store(&state);
    match store.append_review(input, now_ms()) {
        Ok(review) => Json(serde_json::json!({ "review": review })).into_response(),
        Err(err) => json_error(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

fn review_store(state: &AppState) -> ParameterRecommendationReviewStore {
    ParameterRecommendationReviewStore::new(state.config().replay_report_dir.clone())
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

fn json_error(status: StatusCode, error: String) -> axum::response::Response {
    (status, Json(serde_json::json!({ "error": error }))).into_response()
}
