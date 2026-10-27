//! The server settings, ported from `config.service.ts` and `settings.ts`.

use crate::app::{apply, done, log_error, run, Cmd, Model};
use crate::model::WeekNumbering;

/// Loads the server's settings and the property definitions.
pub fn load_server_config() -> Cmd {
    run(|api| async move {
        match api.get_config().await {
            Ok(c) => apply(&api, "config", move |m| {
                m.config.markdown_dir = c.markdown_dir;
                m.config.week_numbering = WeekNumbering::parse(&c.week_numbering);
                m.config.logical_outdent = c.logical_outdent;
                m.config.tags_dir = c.tags_dir;
                m.config.home_link = c.home_link;
                done()
            }),
            Err(e) => log_error(format!("Failed to load config from server: {e}")),
        }
    })
}

pub fn load_property_definitions() -> Cmd {
    run(|api| async move {
        match api.get_property_definitions().await {
            Ok(defs) => apply(&api, "property definitions", move |m| {
                m.config.property_definitions = defs;
                done()
            }),
            Err(e) => log_error(format!("Failed to load property definitions: {e}")),
        }
    })
}

/// Defines a user key. The server checks the key (not a system key, not defined yet, a
/// valid word) and answers with the whole list, or with the reason it was refused.
pub fn add_property_definition(model: &mut Model, key: String, value_type: String) -> Cmd {
    model.config.definition_error = None;
    let key = key.trim().to_string();
    if key.is_empty() {
        model.config.definition_error = Some("Property key is required".into());
        return done();
    }
    run(move |api| async move {
        match api.add_property_definition(&key, &value_type).await {
            Ok(defs) => apply(&api, "property definition added", move |m| {
                m.config.property_definitions = defs;
                done()
            }),
            Err(e) => apply(&api, "property definition refused", move |m| {
                m.config.definition_error = Some(e.0);
                done()
            }),
        }
    })
}

/// Removes a user key with all its values. The server answers with the remaining list, or
/// with the reason it was refused (a system key, an unknown key).
pub fn delete_property_definition(model: &mut Model, key: String) -> Cmd {
    model.config.definition_error = None;
    run(move |api| async move {
        match api.delete_property_definition(&key).await {
            Ok(defs) => apply(&api, "property definition deleted", move |m| {
                m.config.property_definitions = defs;
                done()
            }),
            Err(e) => apply(&api, "property definition delete refused", move |m| {
                m.config.definition_error = Some(e.0);
                done()
            }),
        }
    })
}

pub fn set_week_numbering(model: &mut Model, value: WeekNumbering) -> Cmd {
    model.config.week_numbering = value;
    run(move |api| async move {
        if let Err(e) = api
            .update_config(None, Some(value.as_str().to_string()), None, None, None)
            .await
        {
            log_error(format!("Failed to save config to server: {e}"));
        }
    })
}

pub fn set_logical_outdent(model: &mut Model, value: bool) -> Cmd {
    model.config.logical_outdent = value;
    run(move |api| async move {
        if let Err(e) = api.update_config(None, None, Some(value), None, None).await {
            log_error(format!("Failed to save config to server: {e}"));
        }
    })
}

/// Saves the tag directory. The server normalizes it to the `dirPath` form (`tags/`),
/// so the model takes the server's answer rather than the typed text.
pub fn set_tags_dir(model: &mut Model, value: String) -> Cmd {
    model.config.tags_dir = value.clone();
    run(move |api| async move {
        match api.update_config(None, None, None, Some(value), None).await {
            Ok(c) => apply(&api, "tags dir", move |m| {
                m.config.tags_dir = c.tags_dir;
                done()
            }),
            Err(e) => log_error(format!("Failed to save config to server: {e}")),
        }
    })
}

/// Saves the page the Home button opens. The server keeps the target of a link typed with
/// its brackets, so the model takes the server's answer rather than the typed text.
pub fn set_home_link(model: &mut Model, value: String) -> Cmd {
    model.config.home_link = value.trim().to_string();
    run(move |api| async move {
        match api.update_config(None, None, None, None, Some(value)).await {
            Ok(c) => apply(&api, "home link", move |m| {
                m.config.home_link = c.home_link;
                done()
            }),
            Err(e) => log_error(format!("Failed to save config to server: {e}")),
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
    super::navigate::to_page(
        &super::atlas::folder_path(model, link),
        false,
        false,
        false,
        None,
    )
}

/// Opens the server's folder chooser and saves the chosen markdown directory.
pub fn browse_folder(_model: &mut Model) -> Cmd {
    run(|api| async move {
        match api.browse_folder().await {
            Ok(choice) if choice.selected => {
                let path = choice.path.clone();
                apply(&api, "markdown dir", move |m| {
                    m.config.markdown_dir = path;
                    done()
                });
                if let Err(e) = api
                    .update_config(Some(choice.path), None, None, None, None)
                    .await
                {
                    log_error(format!("Failed to save config to server: {e}"));
                }
            }
            Ok(_) => {}
            Err(e) => log_error(format!("Failed to browse folder: {e}")),
        }
    })
}
