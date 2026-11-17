//! The changes of an atlas page. The journal writes the outline of an atlas page, so every
//! change of it but the place of a card or a callout comes here. Each answers with the items
//! of the page.
//!
//! The atlas service tells the clients of a change itself, so these operations take no
//! `ChangedBy`.

use crate::domain::bounds::Bounds;
use crate::domain::model::Item;
use crate::error::{parse_id, AppResult};

use super::{ItemsAnswer, Journal};
pub use journal_api_types::{AtlasCalloutInput, AtlasGroupInput, AtlasLinkInput, AtlasTitleInput};

impl Journal {
    pub fn add_atlas_link(&self, atlas_id: &str, input: AtlasLinkInput) -> AppResult<ItemsAnswer> {
        let atlas_id = parse_id(atlas_id)?;
        let bounds = input.bounds.parse::<Bounds>()?;
        self.atlas_answer(
            self.services
                .atlas
                .add_link(&atlas_id, &input.target, bounds),
        )
    }

    /// Gives a link card another target.
    pub fn set_atlas_link(
        &self,
        atlas_id: &str,
        item_id: &str,
        input: AtlasLinkInput,
    ) -> AppResult<ItemsAnswer> {
        let atlas_id = parse_id(atlas_id)?;
        let item_id = parse_id(item_id)?;
        self.atlas_answer(
            self.services
                .atlas
                .set_link(&atlas_id, &item_id, &input.target),
        )
    }

    /// Puts the cards into the group of the target card, or makes a group of them with it.
    pub fn group_atlas_cards(
        &self,
        atlas_id: &str,
        input: AtlasGroupInput,
    ) -> AppResult<ItemsAnswer> {
        let atlas_id = parse_id(atlas_id)?;
        self.atlas_answer(
            self.services
                .atlas
                .group(&atlas_id, &input.cards, &input.target),
        )
    }

    pub fn ungroup_atlas_cards(
        &self,
        atlas_id: &str,
        input: AtlasGroupInput,
    ) -> AppResult<ItemsAnswer> {
        let atlas_id = parse_id(atlas_id)?;
        self.atlas_answer(self.services.atlas.ungroup(&atlas_id, &input.cards))
    }

    pub fn set_atlas_group_title(
        &self,
        atlas_id: &str,
        group_id: &str,
        input: AtlasTitleInput,
    ) -> AppResult<ItemsAnswer> {
        let atlas_id = parse_id(atlas_id)?;
        let group_id = parse_id(group_id)?;
        self.atlas_answer(self.services.atlas.set_group_title(
            &atlas_id,
            &group_id,
            &input.side,
            &input.title,
        ))
    }

    pub fn add_atlas_callout(
        &self,
        atlas_id: &str,
        input: AtlasCalloutInput,
    ) -> AppResult<ItemsAnswer> {
        let atlas_id = parse_id(atlas_id)?;
        let bounds = input.bounds.parse::<Bounds>()?;
        self.atlas_answer(self.services.atlas.add_callout(
            &atlas_id,
            bounds,
            &input.side,
            &input.text,
        ))
    }

    /// Gives a callout another side or text. Its place is not changed here.
    pub fn set_atlas_callout(
        &self,
        atlas_id: &str,
        item_id: &str,
        input: AtlasCalloutInput,
    ) -> AppResult<ItemsAnswer> {
        let atlas_id = parse_id(atlas_id)?;
        let item_id = parse_id(item_id)?;
        self.atlas_answer(self.services.atlas.set_callout(
            &atlas_id,
            &item_id,
            &input.side,
            &input.text,
        ))
    }

    /// Removes a card, a group or a callout from the atlas page.
    pub fn remove_atlas_item(&self, atlas_id: &str, item_id: &str) -> AppResult<ItemsAnswer> {
        let atlas_id = parse_id(atlas_id)?;
        let item_id = parse_id(item_id)?;
        self.atlas_answer(self.services.atlas.remove(&atlas_id, &item_id))
    }

    fn atlas_answer(&self, items: AppResult<Vec<Item>>) -> AppResult<ItemsAnswer> {
        Ok(ItemsAnswer {
            items: self.services.dtos.to_dtos(&items?)?,
        })
    }
}
