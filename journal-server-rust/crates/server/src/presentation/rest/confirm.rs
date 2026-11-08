//! The answer of a client to a question that an extension asked (see ADR 009).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde::Deserialize;

use crate::error::{AppError, AppResult};
use crate::state::SharedState;

pub fn routes() -> Router<SharedState> {
    Router::new().route("/api/confirm/{request_id}", post(answer))
}

#[derive(Deserialize)]
struct AnswerRequest {
    ok: bool,
}

/// The first answer wins. A question that was answered already or timed out is not found.
async fn answer(
    State(state): State<SharedState>,
    Path(request_id): Path<String>,
    Json(req): Json<AnswerRequest>,
) -> AppResult<StatusCode> {
    if state.confirm.answer(&request_id, req.ok) {
        Ok(StatusCode::OK)
    } else {
        Err(AppError::not_found(format!(
            "No question waits for an answer: {request_id}"
        )))
    }
}
