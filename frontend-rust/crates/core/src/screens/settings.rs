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

pub fn set_week_numbering(model: &mut Model, value: WeekNumbering) -> Cmd {
    model.config.week_numbering = value;
    run(move |api| async move {
        if let Err(e) = api
            .update_config(None, Some(value.as_str().to_string()), None)
            .await
        {
            log_error(format!("Failed to save config to server: {e}"));
        }
    })
}

pub fn set_logical_outdent(model: &mut Model, value: bool) -> Cmd {
    model.config.logical_outdent = value;
    run(move |api| async move {
        if let Err(e) = api.update_config(None, None, Some(value)).await {
            log_error(format!("Failed to save config to server: {e}"));
        }
    })
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
                if let Err(e) = api.update_config(Some(choice.path), None, None).await {
                    log_error(format!("Failed to save config to server: {e}"));
                }
            }
            Ok(_) => {}
            Err(e) => log_error(format!("Failed to browse folder: {e}")),
        }
    })
}
