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
        .route(
            "/api/config/property-definitions",
            get(property_definitions).post(add_property_definition),
        )
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

/// `propertyDefinitions` is the key -> type map older clients read; `definitions` adds the flags.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PropertyDefinitionsResponse {
    property_definitions: BTreeMap<String, String>,
    definitions: Vec<PropertyDefinitionDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PropertyDefinitionDto {
    key: String,
    value_type: String,
    system: bool,
    page: bool,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct AddPropertyDefinitionRequest {
    key: Option<String>,
    value_type: Option<String>,
    /// Whether the key may be set on a page; true unless given.
    page: Option<bool>,
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

fn definitions_response(state: &SharedState) -> AppResult<PropertyDefinitionsResponse> {
    let definitions = state.properties.find_definitions()?;
    Ok(PropertyDefinitionsResponse {
        property_definitions: definitions
            .iter()
            .map(|d| (d.key.clone(), d.value_type.clone()))
            .collect(),
        definitions: definitions
            .into_iter()
            .map(|d| PropertyDefinitionDto {
                key: d.key,
                value_type: d.value_type,
                system: d.system,
                page: d.page,
            })
            .collect(),
    })
}

async fn property_definitions(State(state): State<SharedState>) -> AppResult<Json<PropertyDefinitionsResponse>> {
    blocking(move || Ok(Json(definitions_response(&state)?))).await
}

async fn add_property_definition(
    State(state): State<SharedState>,
    Json(req): Json<AddPropertyDefinitionRequest>,
) -> AppResult<Json<PropertyDefinitionsResponse>> {
    blocking(move || {
        state.properties.insert_definition(
            req.key.as_deref().unwrap_or("").trim(),
            req.value_type.as_deref().unwrap_or("text"),
            req.page.unwrap_or(true),
        )?;
        Ok(Json(definitions_response(&state)?))
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
