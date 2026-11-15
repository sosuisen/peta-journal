//! The settings and the property definitions.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::AppResult;

use super::{ChangedBy, Journal};

/// Watches the markdown directory for changes made outside the journal. An app with no
/// watcher gives one that does nothing.
pub trait FolderWatch: Send + Sync {
    /// Called when the markdown directory setting changes.
    fn restart(&self);
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigAnswer {
    pub markdown_dir: String,
    pub week_numbering: String,
    pub logical_outdent: bool,
    pub tags_dir: String,
    pub home_link: String,
}

/// The settings to change. A setting not given stays as it is.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct UpdateConfigInput {
    pub markdown_dir: Option<String>,
    pub week_numbering: Option<String>,
    pub logical_outdent: Option<bool>,
    pub tags_dir: Option<String>,
    pub home_link: Option<String>,
}

/// `propertyDefinitions` is the key -> type map older clients read; `definitions` adds the flags.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDefinitionsAnswer {
    pub property_definitions: BTreeMap<String, String>,
    pub definitions: Vec<PropertyDefinitionDto>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PropertyDefinitionDto {
    pub key: String,
    pub value_type: String,
    pub system: bool,
    pub page: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct AddPropertyDefinitionInput {
    pub key: Option<String>,
    pub value_type: Option<String>,
    /// Whether the key may be set on a page; true unless given.
    pub page: Option<bool>,
}

impl Journal {
    pub fn config(&self) -> ConfigAnswer {
        let config = &self.services.config;
        ConfigAnswer {
            markdown_dir: config.markdown_dir(),
            week_numbering: config.week_numbering(),
            logical_outdent: config.logical_outdent(),
            tags_dir: config.tags_dir(),
            home_link: config.home_link(),
        }
    }

    /// Changes the settings given. A new markdown directory that is empty gets all the pages,
    /// and the watcher moves to it.
    pub fn update_config(&self, input: UpdateConfigInput) -> ConfigAnswer {
        let config = &self.services.config;
        if let Some(dir) = &input.markdown_dir {
            config.set_markdown_dir(dir);
            self.services.export.export_all_if_empty(dir);
            self.services.folder_watch.restart();
        }
        if let Some(week_numbering) = &input.week_numbering {
            config.set_week_numbering(week_numbering);
        }
        if let Some(logical_outdent) = input.logical_outdent {
            config.set_logical_outdent(logical_outdent);
        }
        if let Some(tags_dir) = &input.tags_dir {
            config.set_tags_dir(tags_dir);
        }
        if let Some(home_link) = &input.home_link {
            config.set_home_link(home_link);
        }
        self.config()
    }

    pub fn property_definitions(&self) -> AppResult<PropertyDefinitionsAnswer> {
        let definitions = self.services.properties.find_definitions()?;
        Ok(PropertyDefinitionsAnswer {
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

    /// Adds a user key. The type is text unless given.
    pub fn add_property_definition(
        &self,
        input: AddPropertyDefinitionInput,
    ) -> AppResult<PropertyDefinitionsAnswer> {
        self.services.properties.insert_definition(
            input.key.as_deref().unwrap_or("").trim(),
            input.value_type.as_deref().unwrap_or("text"),
            input.page.unwrap_or(true),
        )?;
        self.definitions_changed()?;
        self.property_definitions()
    }

    /// Removes a user key with all its values. The clients are told of the pages that lost a
    /// value, so an open page drops the property without a reload.
    pub fn delete_property_definition(
        &self,
        key: &str,
        by: &ChangedBy,
    ) -> AppResult<PropertyDefinitionsAnswer> {
        let page_ids = self.services.page_service.delete_property_definition(key)?;
        self.definitions_changed()?;
        self.tell(&page_ids, by);
        self.property_definitions()
    }
}
