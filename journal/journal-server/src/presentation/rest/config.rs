//! The settings and the property definitions. The operations are in the journal (ADR 029).
//! Choosing a folder stays here: it opens a dialog of the OS.

use axum::extract::{Path, State};
use axum::{Extension, Json, Router};

use crate::application::folder_chooser;
use crate::error::HttpError;
use crate::presentation::client_id::ClientId;
use crate::presentation::rest::At;
use crate::state::{blocking, SharedState};
use journal_api_types::{routes, FolderChoice};
use journal_core::journal::{
    AddPropertyDefinitionInput, ChangedBy, ConfigAnswer, PropertyDefinitionsAnswer,
    UpdateConfigInput,
};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .at(routes::GET_CONFIG, get_config)
        .at(routes::UPDATE_CONFIG, update_config)
        .at(routes::PROPERTY_DEFINITIONS, property_definitions)
        .at(routes::ADD_PROPERTY_DEFINITION, add_property_definition)
        .at(
            routes::DELETE_PROPERTY_DEFINITION,
            delete_property_definition,
        )
        .at(routes::BROWSE_FOLDER, browse_folder)
}

async fn get_config(State(state): State<SharedState>) -> Json<ConfigAnswer> {
    Json(state.journal.config())
}

async fn update_config(
    State(state): State<SharedState>,
    Json(req): Json<UpdateConfigInput>,
) -> Result<Json<ConfigAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.update_config(req)))).await
}

async fn property_definitions(
    State(state): State<SharedState>,
) -> Result<Json<PropertyDefinitionsAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.property_definitions()?))).await
}

async fn add_property_definition(
    State(state): State<SharedState>,
    Json(req): Json<AddPropertyDefinitionInput>,
) -> Result<Json<PropertyDefinitionsAnswer>, HttpError> {
    blocking(move || Ok(Json(state.journal.add_property_definition(req)?))).await
}

async fn delete_property_definition(
    State(state): State<SharedState>,
    Extension(client): Extension<ClientId>,
    Path(key): Path<String>,
) -> Result<Json<PropertyDefinitionsAnswer>, HttpError> {
    blocking(move || {
        Ok(Json(state.journal.delete_property_definition(
            &key,
            &ChangedBy::Client(client),
        )?))
    })
    .await
}

async fn browse_folder() -> Result<Json<FolderChoice>, HttpError> {
    blocking(move || {
        let choice = folder_chooser::choose_folder();
        Ok(Json(FolderChoice {
            selected: choice.selected,
            path: choice.path,
        }))
    })
    .await
}
