//! JSON shapes of the REST API, identical to the Java server's records.

use std::collections::{BTreeMap, HashSet};

use hmtid::Hmtid;
use serde::Serialize;
use serde_json::Value;

use crate::domain::model::{BacklinkEntry, Item, Page, SearchResult};
use crate::error::AppResult;
use crate::infrastructure::persistence::{ItemRepository, PropertyRepository};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageDto {
    pub id: String,
    pub parent_id: String,
    pub dir_path: String,
    pub name: String,
    #[serde(rename = "type")]
    pub page_type: String,
    pub created_date: i64,
    pub modified_date: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemDto {
    pub id: String,
    pub page_id: String,
    pub parent_item_id: Option<String>,
    pub dir_path: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub created_date: i64,
    pub modified_date: i64,
    pub idx: String,
    pub level: i32,
    pub body: String,
    pub ast: String,
    pub collapsed: bool,
    pub types: Vec<String>,
    pub properties: BTreeMap<String, Value>,
    pub transclude_item_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResultDto {
    pub page_id: String,
    pub page_title: String,
    pub item_id: String,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BacklinkEntryDto {
    pub source_page: PageDto,
    pub source_item: Option<ItemDto>,
}

pub fn page_dto(page: &Page) -> PageDto {
    PageDto {
        id: page.id.to_string(),
        parent_id: page.parent_id.as_ref().map(|p| p.to_string()).unwrap_or_default(),
        dir_path: page.dir_path.clone(),
        name: page.name.clone(),
        page_type: if page.page_type.is_empty() { "doc".into() } else { page.page_type.clone() },
        created_date: page.created_date,
        modified_date: page.modified_date,
    }
}

pub fn item_dto(item: &Item, scalars: Option<&BTreeMap<String, String>>, lists: Option<&BTreeMap<String, Vec<String>>>) -> ItemDto {
    let mut properties: BTreeMap<String, Value> = BTreeMap::new();
    if let Some(s) = scalars {
        for (k, v) in s {
            properties.insert(k.clone(), Value::String(v.clone()));
        }
    }
    if let Some(l) = lists {
        for (k, v) in l {
            properties.insert(
                k.clone(),
                Value::Array(v.iter().map(|x| Value::String(x.clone())).collect()),
            );
        }
    }
    ItemDto {
        id: item.id.to_string(),
        page_id: item.page_id.to_string(),
        parent_item_id: item.parent_item_id.as_ref().map(|p| p.to_string()),
        dir_path: item.dir_path.clone(),
        name: item.name.clone(),
        item_type: if item.item_type.is_empty() { "doc".into() } else { item.item_type.clone() },
        created_date: item.created_date,
        modified_date: item.modified_date,
        idx: item.idx.clone(),
        level: item.level,
        body: item.body.clone(),
        ast: item.ast.clone(),
        collapsed: item.collapsed,
        types: item.types.clone(),
        properties,
        transclude_item_id: item.transclude_item_id.as_ref().map(|p| p.to_string()),
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

/// Only the requested fields of an item DTO, for `?fields=body,level`.
pub fn select_fields(dto: &ItemDto, fields_csv: &str) -> serde_json::Map<String, Value> {
    let requested: HashSet<&str> = fields_csv.split(',').collect();
    let all = serde_json::to_value(dto).unwrap_or(Value::Null);
    let mut out = serde_json::Map::new();
    if let Value::Object(map) = all {
        for (k, v) in map {
            if requested.contains(k.as_str()) {
                out.insert(k, v);
            }
        }
    }
    out
}

/// Builds item DTOs with their properties loaded, and the transcluded trees a list of items refers to.
#[derive(Clone)]
pub struct ItemDtoAssembler {
    properties: PropertyRepository,
    items: ItemRepository,
}

const ITEM_TARGET: &str = "item";
/// How deep a transclusion inside a transclusion is followed.
const MAX_TRANSCLUSION_DEPTH: usize = 5;

impl ItemDtoAssembler {
    pub fn new(properties: PropertyRepository, items: ItemRepository) -> ItemDtoAssembler {
        ItemDtoAssembler { properties, items }
    }

    /// Converts one item, loading its properties.
    pub fn to_dto(&self, item: &Item) -> AppResult<ItemDto> {
        let id = item.id.to_string();
        let scalars = self.properties.find_scalar_by_targets(ITEM_TARGET, std::slice::from_ref(&id))?;
        let lists = self.properties.find_list_by_targets(ITEM_TARGET, std::slice::from_ref(&id))?;
        Ok(item_dto(item, scalars.get(&id), lists.get(&id)))
    }

    /// Converts items in order, loading all their properties at once.
    pub fn to_dtos(&self, items: &[Item]) -> AppResult<Vec<ItemDto>> {
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
    pub fn transcluded_trees(&self, items: &[Item]) -> AppResult<BTreeMap<String, Vec<ItemDto>>> {
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
    ) -> AppResult<()> {
        for item in items {
            let Some(source) = &item.transclude_item_id else { continue };
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
    ) -> AppResult<()> {
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
