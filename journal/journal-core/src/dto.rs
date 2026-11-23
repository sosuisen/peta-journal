//! JSON shapes of the REST API, identical to the Java server's records.

use std::collections::{BTreeMap, HashSet};

use hmtid::Hmtid;
use journal_api_types::PropertyValue;
use serde::Serialize;
use serde_json::Value;

use crate::application::markdown_parser::MarkdownParser;
use crate::domain::model::{BacklinkEntry, Item, Page, SearchResult};
use crate::error::AppError;
use crate::infrastructure::persistence::{ItemRepository, PropertyRepository};

/// A page as the server answers it: the fields the clients read, and the fields only the
/// server has (ADR 082). JSON has them all at one level.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDto {
    #[serde(flatten)]
    pub page: journal_api_types::Page,
    /// The URL that opens the page in the app, encoded, without the origin (ADR 061).
    pub url: String,
}

/// An item as the server answers it: the fields the clients read, and the fields only the
/// server has (ADR 082). JSON has them all at one level.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    #[serde(flatten)]
    pub item: journal_api_types::Item,
    pub parent_item_id: Option<String>,
    pub ast: String,
    /// The URL that opens the page of the item at the item, encoded, without the origin
    /// (ADR 061).
    pub url: String,
}

/// A search result has the same fields on the server and the clients, so they share one type
/// (ADR 082).
pub use journal_api_types::SearchResult as SearchResultDto;

/// A page that links to another, and the item that holds the link. The server and the clients share the type (ADR 082).
pub type BacklinkEntryDto = journal_api_types::BacklinkEntry<PageDto, ItemDto>;

/// A page without its properties, for lists of pages.
pub fn page_dto(page: &Page) -> PageDto {
    page_dto_with(page, None, None)
}

pub fn page_dto_with(
    page: &Page,
    scalars: Option<&BTreeMap<String, String>>,
    lists: Option<&BTreeMap<String, Vec<String>>>,
) -> PageDto {
    PageDto {
        page: journal_api_types::Page {
            id: page.id.to_string(),
            parent_id: page
                .parent_id
                .as_ref()
                .map(|p| p.to_string())
                .unwrap_or_default(),
            dir_path: page.dir_path.clone(),
            name: page.name.clone(),
            page_type: if page.page_type.is_empty() {
                "doc".into()
            } else {
                page.page_type.clone()
            },
            created_date: page.created_date,
            modified_date: page.modified_date,
            properties: properties_json(scalars, lists),
        },
        url: page_urls::page_url(&format!("{}{}", page.dir_path, page.name)),
    }
}

fn properties_json(
    scalars: Option<&BTreeMap<String, String>>,
    lists: Option<&BTreeMap<String, Vec<String>>>,
) -> BTreeMap<String, PropertyValue> {
    let mut properties = BTreeMap::new();
    if let Some(s) = scalars {
        for (k, v) in s {
            properties.insert(k.clone(), PropertyValue::Text(v.clone()));
        }
    }
    if let Some(l) = lists {
        for (k, v) in l {
            properties.insert(k.clone(), PropertyValue::List(v.clone()));
        }
    }
    properties
}

pub fn item_dto(
    item: &Item,
    scalars: Option<&BTreeMap<String, String>>,
    lists: Option<&BTreeMap<String, Vec<String>>>,
) -> ItemDto {
    let properties = properties_json(scalars, lists);
    ItemDto {
        item: journal_api_types::Item {
            id: item.id.to_string(),
            page_id: item.page_id.to_string(),
            dir_path: item.dir_path.clone(),
            name: item.name.clone(),
            item_type: if item.item_type.is_empty() {
                "doc".into()
            } else {
                item.item_type.clone()
            },
            created_date: item.created_date,
            modified_date: item.modified_date,
            idx: item.idx.clone(),
            level: item.level,
            body: item.body.clone(),
            collapsed: item.collapsed,
            types: item.types.clone(),
            properties,
            transclude_item_id: item.transclude_item_id.as_ref().map(|p| p.to_string()),
        },
        parent_item_id: item.parent_item_id.as_ref().map(|p| p.to_string()),
        // The journal keeps no AST. The apps that do not parse a body themselves read it here
        ast: MarkdownParser.parse_to_ast(&item.body),
        url: page_urls::item_url(&format!("{}{}", item.dir_path, item.name), item.id.as_ref()),
    }
}

pub fn search_result_dto(r: &SearchResult) -> SearchResultDto {
    SearchResultDto {
        page_id: r.page_id.to_string(),
        page_title: r.page_title.clone(),
        item_id: r.item_id.clone(),
        snippet: r.snippet.clone(),
        score: r.score,
    }
}

pub fn backlink_dto(e: &BacklinkEntry) -> BacklinkEntryDto {
    BacklinkEntryDto {
        source_page: page_dto(&e.source_page),
        source_item: e.source_item.as_ref().map(|i| item_dto(i, None, None)),
    }
}

/// An item DTO with only the fields a client asked for.
pub type NarrowItem = serde_json::Map<String, Value>;

/// The fields of an item a client asked for, as in `?fields=body,level`.
pub struct ItemFields(HashSet<String>);

impl ItemFields {
    /// The fields named in a comma-separated list. `None` when no field is named: the client
    /// then gets whole items.
    pub fn parse(fields_csv: Option<&str>) -> Option<ItemFields> {
        let csv = fields_csv.filter(|csv| !csv.is_empty())?;
        Some(ItemFields(csv.split(',').map(String::from).collect()))
    }

    /// Only these fields of an item DTO.
    pub fn of(&self, dto: &ItemDto) -> NarrowItem {
        let all = serde_json::to_value(dto).unwrap_or(Value::Null);
        let mut out = serde_json::Map::new();
        if let Value::Object(map) = all {
            for (k, v) in map {
                if self.0.contains(&k) {
                    out.insert(k, v);
                }
            }
        }
        out
    }

    /// Only these fields of each item DTO, in the same order.
    pub fn of_each(&self, dtos: &[ItemDto]) -> Vec<NarrowItem> {
        dtos.iter().map(|dto| self.of(dto)).collect()
    }
}

/// Builds item DTOs with their properties loaded, and the transcluded trees a list of items refers to.
#[derive(Clone)]
pub struct ItemDtoAssembler {
    properties: PropertyRepository,
    items: ItemRepository,
}

const ITEM_TARGET: &str = "item";
const PAGE_TARGET: &str = "page";
/// How deep a transclusion inside a transclusion is followed.
const MAX_TRANSCLUSION_DEPTH: usize = 5;

impl ItemDtoAssembler {
    pub fn new(properties: PropertyRepository, items: ItemRepository) -> ItemDtoAssembler {
        ItemDtoAssembler { properties, items }
    }

    /// Converts one page, loading its properties.
    pub fn page_to_dto(&self, page: &Page) -> Result<PageDto, AppError> {
        let id = page.id.to_string();
        let scalars = self
            .properties
            .find_scalar_by_targets(PAGE_TARGET, std::slice::from_ref(&id))?;
        let lists = self
            .properties
            .find_list_by_targets(PAGE_TARGET, std::slice::from_ref(&id))?;
        Ok(page_dto_with(page, scalars.get(&id), lists.get(&id)))
    }

    /// Converts one item, loading its properties.
    pub fn to_dto(&self, item: &Item) -> Result<ItemDto, AppError> {
        let id = item.id.to_string();
        let scalars = self
            .properties
            .find_scalar_by_targets(ITEM_TARGET, std::slice::from_ref(&id))?;
        let lists = self
            .properties
            .find_list_by_targets(ITEM_TARGET, std::slice::from_ref(&id))?;
        Ok(item_dto(item, scalars.get(&id), lists.get(&id)))
    }

    /// Converts items in order, loading all their properties at once.
    pub fn to_dtos(&self, items: &[Item]) -> Result<Vec<ItemDto>, AppError> {
        let ids: Vec<String> = items.iter().map(|i| i.id.to_string()).collect();
        let scalars = self.properties.find_scalar_by_targets(ITEM_TARGET, &ids)?;
        let lists = self.properties.find_list_by_targets(ITEM_TARGET, &ids)?;
        Ok(items
            .iter()
            .map(|item| {
                let id = item.id.to_string();
                item_dto(item, scalars.get(&id), lists.get(&id))
            })
            .collect())
    }

    /// The source trees of every transclusion among the items, and of transclusions found inside
    /// those trees, up to `MAX_TRANSCLUSION_DEPTH` levels deep. A source that no longer exists maps
    /// to an empty list.
    pub fn transcluded_trees(
        &self,
        items: &[Item],
    ) -> Result<BTreeMap<String, Vec<ItemDto>>, AppError> {
        let mut trees = BTreeMap::new();
        let mut visited = HashSet::new();
        self.load_trees(items, &mut trees, &mut visited, 0)?;
        Ok(trees)
    }

    fn load_trees(
        &self,
        items: &[Item],
        trees: &mut BTreeMap<String, Vec<ItemDto>>,
        visited: &mut HashSet<String>,
        depth: usize,
    ) -> Result<(), AppError> {
        for item in items {
            let Some(source) = &item.transclude_item_id else {
                continue;
            };
            let source_id = source.to_string();
            if !trees.contains_key(&source_id) {
                self.load_tree(source, trees, visited, depth)?;
            }
        }
        Ok(())
    }

    fn load_tree(
        &self,
        source: &Hmtid,
        trees: &mut BTreeMap<String, Vec<ItemDto>>,
        visited: &mut HashSet<String>,
        depth: usize,
    ) -> Result<(), AppError> {
        let source_id = source.to_string();
        if depth >= MAX_TRANSCLUSION_DEPTH || visited.contains(&source_id) {
            return Ok(());
        }
        visited.insert(source_id.clone());
        let descendants = self.items.find_item_with_descendants(source)?;
        if descendants.is_empty() {
            trees.insert(source_id, Vec::new());
            return Ok(());
        }
        trees.insert(source_id, self.to_dtos(&descendants)?);
        self.load_trees(&descendants, trees, visited, depth + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(text: &str) -> Hmtid {
        Hmtid::parse(text).unwrap()
    }

    // ページの答えのJSONは、共有の欄とサーバーだけの欄を、同じ階層に並べる。
    #[test]
    fn the_json_of_a_page_has_the_shared_fields_and_the_server_fields_at_one_level() {
        let page = Page::new(id("2026-11-24-00-00-00-AAAAAAA"), "d/", "n", "doc", 1);
        let lists = BTreeMap::from([("tags".to_string(), vec!["x".to_string()])]);

        let json = serde_json::to_value(page_dto_with(&page, None, Some(&lists))).unwrap();

        let mut keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "createdDate",
                "dirPath",
                "id",
                "modifiedDate",
                "name",
                "parentId",
                "properties",
                "type",
                "url"
            ]
        );
        assert_eq!(json["properties"], serde_json::json!({ "tags": ["x"] }));
    }

    // 項目の答えのJSONは、共有の欄とサーバーだけの欄を、同じ階層に並べる。
    #[test]
    fn the_json_of_an_item_has_the_shared_fields_and_the_server_fields_at_one_level() {
        let page = Page::new(id("2026-11-24-00-00-00-AAAAAAA"), "d/", "n", "doc", 1);
        let item = Item::new_on(id("2026-11-24-00-00-01-AAAAAAA"), &page, 1);
        let scalars = BTreeMap::from([("plan".to_string(), "2026-11-24".to_string())]);
        let lists = BTreeMap::from([("tags".to_string(), vec!["x".to_string()])]);

        let json = serde_json::to_value(item_dto(&item, Some(&scalars), Some(&lists))).unwrap();

        let mut keys: Vec<&str> = json
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "ast",
                "body",
                "collapsed",
                "createdDate",
                "dirPath",
                "id",
                "idx",
                "level",
                "modifiedDate",
                "name",
                "pageId",
                "parentItemId",
                "properties",
                "transcludeItemId",
                "type",
                "types",
                "url"
            ]
        );
        assert_eq!(
            json["properties"],
            serde_json::json!({ "plan": "2026-11-24", "tags": ["x"] })
        );
    }
}
