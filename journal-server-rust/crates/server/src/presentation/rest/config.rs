use std::collections::BTreeMap;

use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::application::folder_chooser;
use crate::error::AppResult;
use crate::infrastructure::persistence::property_repository::is_atlas_key;
use crate::state::{blocking, SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/api/config", get(get_config).put(update_config))
        .route(
            "/api/config/property-definitions",
            get(property_definitions).post(add_property_definition),
        )
        .route("/api/config/property-definitions/{key}", delete(delete_property_definition))
        .route("/api/config/browse-folder", post(browse_folder))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigResponse {
    markdown_dir: String,
    week_numbering: String,
    logical_outdent: bool,
    tags_dir: String,
    home_link: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct UpdateConfigRequest {
    markdown_dir: Option<String>,
    week_numbering: Option<String>,
    logical_outdent: Option<bool>,
    tags_dir: Option<String>,
    home_link: Option<String>,
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
        tags_dir: state.config.tags_dir(),
        home_link: state.config.home_link(),
    }
}

async fn get_config(State(state): State<SharedState>) -> Json<ConfigResponse> {
    Json(current(&state))
}

fn definitions_response(state: &SharedState) -> AppResult<PropertyDefinitionsResponse> {
    // A card group of an atlas and its title are not keys the user chooses; there may be many
    let definitions: Vec<_> = state
        .properties
        .find_definitions()?
        .into_iter()
        .filter(|d| !is_atlas_key(&d.key))
        .collect();
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

/// Removes a user key with all its values. The pages that lost a value are announced over
/// SSE, so an open page drops the property without a reload.
async fn delete_property_definition(
    State(state): State<SharedState>,
    Path(key): Path<String>,
) -> AppResult<Json<PropertyDefinitionsResponse>> {
    blocking(move || {
        for page_id in state.page_service.delete_property_definition(&key)? {
            state.sse.broadcast_pages_updated(page_id.as_str());
        }
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
        if let Some(t) = &req.tags_dir {
            state.config.set_tags_dir(t);
        }
        if let Some(h) = &req.home_link {
            state.config.set_home_link(h);
        }
        Ok(Json(current(&state)))
    })
    .await
}
