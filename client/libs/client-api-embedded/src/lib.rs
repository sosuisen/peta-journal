//! Performs [`ApiOp`]s with a journal core inside the app, where `client-api-http` sends
//! them to the journal server (ADR 029).
//!
//! The journal answers in the JSON shapes of the REST API. So an answer is written as JSON
//! and read by the same code of the frontend core that reads the answer of the server. The
//! frontend core cannot tell the two apart.

mod events;

pub use events::EventQueue;

use std::path::Path;
use std::sync::Arc;

use client_core::effects::{ApiError, ApiOp, ApiOutput, ApiResult, AtlasChange};
use client_core::rest::{rest_call, ClientId, HttpReply};
use journal_core::application::confirm_service::ConfirmService;
use journal_core::domain::client_id::ClientId as JournalClientId;
use journal_core::error::AppError;
use journal_core::journal::{ChangedBy, FindPageQuery, Journal};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

/// The journal of this app, as the frontend core calls it.
pub struct EmbeddedJournal {
    journal: Arc<Journal>,
    confirm: Arc<ConfirmService>,
    /// The changes of this frontend carry its ID, as its requests to a server would
    by: ChangedBy,
}

impl EmbeddedJournal {
    pub fn new(
        journal: Arc<Journal>,
        confirm: Arc<ConfirmService>,
        client_id: &ClientId,
    ) -> EmbeddedJournal {
        let client: JournalClientId = client_id
            .as_str()
            .parse()
            .expect("the ID of a frontend is a UUID");
        EmbeddedJournal {
            journal,
            confirm,
            by: ChangedBy::Client(client),
        }
    }

    /// Performs one operation and answers it the way the core expects.
    pub fn call(&self, op: ApiOp) -> ApiResult<ApiOutput> {
        // Why the REST call: it has the body of the request, which the journal reads as its
        // input, and the reader of the answer. An upload has none: only the shell knows its file
        let rest = rest_call(op.clone()).ok_or_else(|| ApiError("not a REST call".into()))?;
        let reply = match self.answer(op, rest.request.body.clone()) {
            Ok(body) => HttpReply {
                status: 200,
                body: body.to_string(),
            },
            Err(error) => HttpReply {
                status: error.http_status(),
                body: json!({ "error": error.to_string() }).to_string(),
            },
        };
        rest.decode(&reply)
    }

    /// Copies the file at `path` into the assets, for an image the shell pasted (ADR 071).
    /// The core asks with a number that only the shell knows, so the shell gives the path.
    pub fn upload(&self, path: &Path) -> ApiResult<ApiOutput> {
        let saved = self
            .journal
            .save_asset_file(&path.to_string_lossy())
            .map_err(|e| ApiError(e.to_string()))?;
        let uploaded = serde_json::to_value(saved)
            .and_then(serde_json::from_value)
            .map_err(|e| ApiError(e.to_string()))?;
        Ok(ApiOutput::Uploaded(uploaded))
    }

    /// The answer of the journal as JSON. `body` is the body of the REST request.
    fn answer(&self, op: ApiOp, body: Option<Value>) -> Result<Value, AppError> {
        let journal = &self.journal;
        let by = &self.by;
        match op {
            // ---- pages ----
            ApiOp::GetPageById { page_id } => json_of(journal.get_page(&page_id)?),
            ApiOp::FindPage {
                journal: name,
                title,
                dir_path,
            } => json_of(journal.find_page(FindPageQuery {
                journal: name,
                title: Some(title),
                dir_path: Some(dir_path),
            })?),
            ApiOp::GetOrCreateDoc { title, dir_path } => {
                json_of(journal.get_or_create_doc(Some(&title), Some(&dir_path))?)
            }
            ApiOp::UpdatePage {
                page_id,
                title,
                dir_path,
                overwrite,
            } => json_of(journal.update_page(
                &page_id,
                title.as_deref(),
                dir_path.as_deref(),
                overwrite,
                by,
            )?),
            ApiOp::DemotePage { page_id, overwrite } => {
                json_of(journal.demote_page(&page_id, overwrite, by)?)
            }
            ApiOp::UpsertPageProperty {
                page_id,
                key,
                value_type,
                value,
            } => json_of(journal.set_page_property(
                &page_id,
                &key,
                Some(&value_type),
                Some(&value),
                by,
            )?),
            ApiOp::DeletePageProperty { page_id, key } => {
                json_of(journal.delete_page_property(&page_id, &key, by)?)
            }
            ApiOp::DeletePage { page_id } => {
                journal.delete_page(&page_id, by)?;
                Ok(Value::Null)
            }
            ApiOp::GetRecentPages {
                max_results,
                page_type,
            } => json_of(journal.recent_pages(Some(max_results.into()), page_type.as_deref())?),
            ApiOp::GetBacklinks { page_id } => json_of(journal.backlinks(&page_id)?),
            ApiOp::GetChildPages { parent_id } => json_of(journal.child_pages(&parent_id)?),

            // ---- journals ----
            ApiOp::GetOrCreateJournal { file_name } => {
                json_of(journal.get_or_create_journal(&file_name)?)
            }
            ApiOp::MoveItemsToJournal { file_name, .. } => {
                json_of(journal.move_items_to_journal(&file_name, input(body)?, by)?)
            }
            ApiOp::GetMostRecentJournalBefore { before } => {
                json_of(journal.most_recent_journal_before(&before)?)
            }
            ApiOp::GetMostRecentWeeklyJournalBefore { before } => {
                json_of(journal.most_recent_weekly_journal_before(&before)?)
            }

            // ---- items ----
            ApiOp::CreateItem { .. } => json_of(journal.create_item(input(body)?, by)?),
            ApiOp::UpdateItem { item_id, .. } => {
                json_of(journal.update_item(&item_id, input(body)?, by)?)
            }
            ApiOp::ShiftItemLevels { .. } => json_of(journal.shift_levels(input(body)?, by)?),
            ApiOp::MoveItems { .. } => json_of(journal.move_items(input(body)?, by)?),
            ApiOp::DeleteItems { .. } => json_of(journal.delete_items(input(body)?, by)?),
            ApiOp::MergeItems { .. } => json_of(journal.merge_items(input(body)?, by)?),
            ApiOp::PasteText { item_id, .. } => {
                json_of(journal.paste(&item_id, input(body)?, by)?)
            }
            ApiOp::SplitItem { item_id, .. } => {
                json_of(journal.split(&item_id, input(body)?, by)?)
            }
            ApiOp::StashItem { item_id, .. } => {
                json_of(journal.stash(&item_id, input(body)?, by)?)
            }
            ApiOp::ApplyBatch { .. } => json_of(journal.apply_batch(input(body)?, by)?),
            ApiOp::CycleTask { item_id } => json_of(journal.cycle_task(&item_id, by)?),
            ApiOp::UpsertItemProperty { item_id, key, .. } => {
                json_of(journal.set_item_property(&item_id, &key, input(body)?, by)?)
            }
            ApiOp::DeleteItemProperty { item_id, key } => {
                json_of(journal.delete_item_property(&item_id, &key, by)?)
            }
            ApiOp::ExportTreeMarkdown { .. } => {
                json_of(journal.export_tree_markdown(input(body)?)?)
            }
            ApiOp::TranscludedItems { .. } => json_of(journal.transcluded_items(input(body)?)?),
            ApiOp::SetTransclusion { item_id, .. } => {
                json_of(journal.set_transclusion(&item_id, input(body)?, by)?)
            }
            ApiOp::RemoveTransclusion { item_id } => {
                json_of(journal.remove_transclusion(&item_id, by)?)
            }

            // ---- atlas ----
            ApiOp::ChangeAtlas { atlas_id, change } => match change {
                AtlasChange::AddLink { .. } => {
                    json_of(journal.add_atlas_link(&atlas_id, input(body)?)?)
                }
                AtlasChange::SetLink { item_id, .. } => {
                    json_of(journal.set_atlas_link(&atlas_id, &item_id, input(body)?)?)
                }
                AtlasChange::Group { .. } => {
                    json_of(journal.group_atlas_cards(&atlas_id, input(body)?)?)
                }
                AtlasChange::Ungroup { .. } => {
                    json_of(journal.ungroup_atlas_cards(&atlas_id, input(body)?)?)
                }
                AtlasChange::SetGroupTitle { item_id, .. } => {
                    json_of(journal.set_atlas_group_title(&atlas_id, &item_id, input(body)?)?)
                }
                AtlasChange::AddCallout { .. } => {
                    json_of(journal.add_atlas_callout(&atlas_id, input(body)?)?)
                }
                AtlasChange::SetCallout { item_id, .. } => {
                    json_of(journal.set_atlas_callout(&atlas_id, &item_id, input(body)?)?)
                }
                AtlasChange::Remove { item_id } => {
                    json_of(journal.remove_atlas_item(&atlas_id, &item_id)?)
                }
            },

            // ---- search ----
            ApiOp::SearchCombined { query, max_results } => {
                json_of(journal.search_combined(&query, Some(max_results as usize))?)
            }
            ApiOp::SearchPageNames { query, max_results } => {
                json_of(journal.search_page_names(&query, Some(max_results as usize))?)
            }

            // ---- settings ----
            ApiOp::GetConfig => json_of(journal.config()),
            ApiOp::UpdateConfig { .. } => json_of(journal.update_config(input(body)?)),
            ApiOp::GetPropertyDefinitions => json_of(journal.property_definitions()?),
            ApiOp::AddPropertyDefinition { .. } => {
                json_of(journal.add_property_definition(input(body)?)?)
            }
            ApiOp::DeletePropertyDefinition { key } => {
                json_of(journal.delete_property_definition(&key, by)?)
            }

            // ---- extensions ----
            ApiOp::AnswerConfirm { request_id, ok } => {
                if self.confirm.answer(&request_id, ok) {
                    Ok(Value::Null)
                } else {
                    Err(AppError::not_found(format!(
                        "No question waits for an answer: {request_id}"
                    )))
                }
            }

            // What the journal server does with the machine it runs on. An app does these
            // itself, with what its platform gives it
            ApiOp::BrowseFolder | ApiOp::OpenExportFolder { .. } | ApiOp::UploadAsset { .. } => {
                Err(AppError::not_found(
                    "The journal inside the app does not do this",
                ))
            }
        }
    }
}

fn json_of(answer: impl Serialize) -> Result<Value, AppError> {
    Ok(serde_json::to_value(answer)?)
}

/// The input of an operation, read from the body of the REST request.
fn input<T: DeserializeOwned>(body: Option<Value>) -> Result<T, AppError> {
    serde_json::from_value(body.unwrap_or(Value::Null))
        .map_err(|e| AppError::bad_request(e.to_string()))
}
