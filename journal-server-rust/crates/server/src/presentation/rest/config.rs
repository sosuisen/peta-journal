use std::collections::BTreeMap;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::application::folder_chooser;
use crate::error::AppResult;
use crate::state::{blocking, SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/config", get(get_config).put(update_config))
        .route("/api/config/property-definitions", get(property_definitions))
        .route("/api/config/browse-folder", post(browse_folder))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigResponse {
    markdown_dir: String,
    week_numbering: String,
    logical_outdent: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct UpdateConfigRequest {
    markdown_dir: Option<String>,
    week_numbering: Option<String>,
    logical_outdent: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PropertyDefinitionsResponse {
    property_definitions: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct BrowseFolderResponse {
    selected: bool,
    path: String,
}

fn current(state: &SharedState) -> ConfigResponse {
    ConfigResponse {
        markdown_dir: state.config.markdown_dir(),
        week_numbering: state.config.week_numbering(),
        logical_outdent: state.config.logical_outdent(),
    }
}

async fn get_config(State(state): State<SharedState>) -> Json<ConfigResponse> {
    Json(current(&state))
}

async fn property_definitions(State(state): State<SharedState>) -> AppResult<Json<PropertyDefinitionsResponse>> {
    blocking(move || {
        Ok(Json(PropertyDefinitionsResponse {
            property_definitions: state.properties.find_all_definitions()?,
        }))
    })
    .await
}

async fn browse_folder() -> AppResult<Json<BrowseFolderResponse>> {
    blocking(move || {
        let choice = folder_chooser::choose_folder();
        Ok(Json(BrowseFolderResponse {
            selected: choice.selected,
            path: choice.path,
        }))
    })
    .await
}

async fn update_config(State(state): State<SharedState>, Json(req): Json<UpdateConfigRequest>) -> AppResult<Json<ConfigResponse>> {
    blocking(move || {
        if let Some(dir) = &req.markdown_dir {
            state.config.set_markdown_dir(dir);
            state.export.export_all_if_empty(dir);
            state.watcher.restart();
        }
        if let Some(w) = &req.week_numbering {
            state.config.set_week_numbering(w);
        }
        if let Some(l) = req.logical_outdent {
            state.config.set_logical_outdent(l);
        }
        Ok(Json(current(&state)))
    })
    .await
}
