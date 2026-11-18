//! The answer of a client to a question that an extension asked (see ADR 009).

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Json, Router};
use journal_api_types::routes;
use journal_api_types::AnswerConfirmInput;

use crate::error::{AppError, HttpResult};
use crate::presentation::rest::At;
use crate::state::SharedState;

pub fn routes() -> Router<SharedState> {
    Router::new().at(routes::ANSWER_CONFIRM, answer)
}

/// The first answer wins. A question that was answered already or timed out is not found.
async fn answer(
    State(state): State<SharedState>,
    Path(request_id): Path<String>,
    Json(req): Json<AnswerConfirmInput>,
) -> HttpResult<StatusCode> {
    if state.confirm.answer(&request_id, req.ok) {
        Ok(StatusCode::OK)
    } else {
        Err(AppError::not_found(format!("No question waits for an answer: {request_id}")).into())
    }
}
