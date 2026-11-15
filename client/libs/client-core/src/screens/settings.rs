//! The server settings, ported from `config.service.ts` and `settings.ts`.

use crate::app::{done, log_error, run, Cmd, Event, Model};
use crate::effects::{Api, ApiResult};
use crate::model::{PropertyDefinition, ServerConfig, WeekNumbering, USER_VALUE_TYPES};
use property_names::name_of;

/// Loads the server's settings and the property definitions.
pub fn load_server_config() -> Cmd {
    run(|api| async move {
        match api.get_config().await {
            Ok(config) => api.send_event(Event::ConfigLoaded { config }),
            Err(e) => log_error(format!("Failed to load config from server: {e}")),
        }
    })
}

/// Takes the settings the server answered with.
pub fn show_config(model: &mut Model, c: ServerConfig) -> Cmd {
    model.config.markdown_dir = c.markdown_dir;
    model.config.week_numbering = WeekNumbering::parse(&c.week_numbering);
    model.config.logical_outdent = c.logical_outdent;
    model.config.tags_dir = c.tags_dir;
    model.config.home_link = c.home_link;
    done()
}

pub fn load_property_definitions() -> Cmd {
    run(|api| async move {
        match api.get_property_definitions().await {
            Ok(definitions) => api.send_event(Event::PropertyDefinitionsLoaded { definitions }),
            Err(e) => log_error(format!("Failed to load property definitions: {e}")),
        }
    })
}

/// Defines a user key, named by the body the user typed with the suffix of its type (ADR
/// 047). The server checks the key (not a system key, not defined yet, a valid word) and
/// answers with the whole list, or with the reason it was refused.
pub fn add_property_definition(model: &mut Model, body: String, value_type: String) -> Cmd {
    model.config.definition_error = None;
    let body = body.trim();
    if body.is_empty() {
        model.config.definition_error = Some("Property key is required".into());
        return done();
    }
    let Some(t) = USER_VALUE_TYPES
        .into_iter()
        .find(|t| t.as_str() == value_type)
    else {
        model.config.definition_error = Some(format!("A key cannot be of type {value_type}"));
        return done();
    };
    let Ok(key) = name_of(body, t) else {
        model.config.definition_error =
            Some("The name cannot end with _n, _d, _dt, _cb, _bounds or _list".into());
        return done();
    };
    run(move |api| async move {
        settle_definitions(&api, api.add_property_definition(&key, &value_type).await);
    })
}

/// Removes a user key with all its values. The server answers with the remaining list, or
/// with the reason it was refused (a system key, an unknown key).
pub fn delete_property_definition(model: &mut Model, key: String) -> Cmd {
    model.config.definition_error = None;
    run(move |api| async move {
        settle_definitions(&api, api.delete_property_definition(&key).await);
    })
}

/// Takes the server's answer to a change of the property definitions: the whole list, or
/// the reason the change was refused.
fn settle_definitions(api: &Api<Event>, result: ApiResult<Vec<PropertyDefinition>>) {
    match result {
        Ok(definitions) => api.send_event(Event::PropertyDefinitionsLoaded { definitions }),
        Err(e) => api.send_event(Event::PropertyDefinitionRefused { reason: e.0 }),
    }
}

pub fn set_week_numbering(model: &mut Model, value: WeekNumbering) -> Cmd {
    model.config.week_numbering = value;
    run(move |api| async move {
        save_config(
            &api,
            None,
            Some(value.as_str().to_string()),
            None,
            None,
            None,
        )
        .await;
    })
}

pub fn set_logical_outdent(model: &mut Model, value: bool) -> Cmd {
    model.config.logical_outdent = value;
    run(move |api| async move {
        save_config(&api, None, None, Some(value), None, None).await;
    })
}

/// Saves the given settings on the server. A failure is logged: the screen keeps the
/// typed value. The server's config comes back when it was saved.
async fn save_config(
    api: &Api<Event>,
    markdown_dir: Option<String>,
    week_numbering: Option<String>,
    logical_outdent: Option<bool>,
    tags_dir: Option<String>,
    home_link: Option<String>,
) -> Option<ServerConfig> {
    match api
        .update_config(
            markdown_dir,
            week_numbering,
            logical_outdent,
            tags_dir,
            home_link,
        )
        .await
    {
        Ok(config) => Some(config),
        Err(e) => {
            log_error(format!("Failed to save config to server: {e}"));
            None
        }
    }
}

/// Saves the tag directory. The server normalizes it to the `dirPath` form (`tags/`),
/// so the model takes the server's answer rather than the typed text.
pub fn set_tags_dir(model: &mut Model, value: String) -> Cmd {
    model.config.tags_dir = value.clone();
    run(move |api| async move {
        if let Some(c) = save_config(&api, None, None, None, Some(value), None).await {
            api.send_event(Event::TagsDirSaved {
                tags_dir: c.tags_dir,
            });
        }
    })
}

/// Saves the page the Home button opens. The server keeps the target of a link typed with
/// its brackets, so the model takes the server's answer rather than the typed text.
pub fn set_home_link(model: &mut Model, value: String) -> Cmd {
    model.config.home_link = value.trim().to_string();
    run(move |api| async move {
        if let Some(c) = save_config(&api, None, None, None, None, Some(value)).await {
            api.send_event(Event::HomeLinkSaved {
                home_link: c.home_link,
            });
        }
    })
}

/// Opens the page of the Home button: the page of its link, or today's journal without one.
/// A link to a folder's index opens the folder as it was last seen, as the root button does.
pub fn navigate_home(model: &mut Model) -> Cmd {
    let link = model.config.home_link.trim();
    if link.is_empty() {
        return super::navigate::to_url("/today", false);
    }
    super::navigate::open_folder(model, link)
}

/// Opens the server's folder chooser and saves the chosen markdown directory.
pub fn browse_folder(_model: &mut Model) -> Cmd {
    run(|api| async move {
        match api.browse_folder().await {
            Ok(choice) if choice.selected => {
                api.send_event(Event::MarkdownDirChosen {
                    path: choice.path.clone(),
                });
                save_config(&api, Some(choice.path), None, None, None, None).await;
            }
            Ok(_) => {}
            Err(e) => log_error(format!("Failed to browse folder: {e}")),
        }
    })
}
