//! U7: story planet / slot browser data. Reads every magic-confirmed SLOT_LIST in the
//! catalog (U4 reader), names each slot through the English translation tables (U1) and
//! decodes the slot badge icons. Renderer-independent: the viewer draws the result.
//!
//! Rules (independent notes, ennuo/toolkit d6a2ef80b5a09999f08f23a803b516096055e5c9,
//! lib/cwlib/src/main/java/cwlib):
//! - Display name: `Slot.toString`: a non-empty translation tag gives
//!   `translate(tag + "_NAME")`, else the tag itself; no tag gives the stored name, else
//!   "Unnamed Level". CONFIRMED_FROM_COMMUNITY_CODE.
//! - Description: `translate(tag + "_DESC")`, else the stored description. The `_DESC`
//!   suffix is named in Toolkit only as a convention (docs/ui-research.md);
//!   STRONG_INFERENCE, counted separately.
//! - Tables: g54800 (`E_AMERICAN_TRANS`) is the base table and g69910
//!   (`E_AMERICAN_PATCH_TRANS`) is patched over it, the patch winning per key
//!   (`tools/unlockall/RestlessPopulator.java`, `RTranslationTable.patch`).
//!   CONFIRMED_FROM_COMMUNITY_CODE; which file paths hold them is not established, so rows
//!   are selected by GUID and must have a `.trans` path. No other language is guessed.
//! - Lock state shown is `initiallyLocked` (disc default). The live state is profile data,
//!   which is not read here.
use crate::level_scene::read_resource;
use lbp_catalog::{attachment::AttachedSource, Catalog, LoadControl};
use lbp_level::serial::Descriptor;
pub use lbp_level::slots::SlotId;
use lbp_level::slots::{self, Slot, SlotList, UiData};
use lbp_level::Limits;
use lbp_resource::translation::{parse_translation_table, TranslationLimits, TranslationTable};
use std::collections::{BTreeMap, BTreeSet};

/// `E_AMERICAN_TRANS` and `E_AMERICAN_PATCH_TRANS` in Toolkit.
pub const BASE_TRANSLATION_GUID: u32 = 54800;
pub const PATCH_TRANSLATION_GUID: u32 = 69910;
const MAX_TRANSLATION_BYTES: usize = 32 << 20;
/// Distinct icons decoded per story, and their total RGBA bytes.
pub const MAX_ICONS: usize = 4096;
pub const MAX_ICON_BYTES: usize = 512 << 20;

/// A resource reference as the browser keys it (GUID preferred, as the catalog resolves).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResourceKey {
    Guid(u32),
    Hash([u8; 20]),
}

impl ResourceKey {
    pub fn of(d: &Descriptor) -> Option<Self> {
        match (d.guid, d.sha1) {
            (Some(g), _) if g != 0 => Some(Self::Guid(g)),
            (_, Some(h)) if h != [0; 20] => Some(Self::Hash(h)),
            _ => None,
        }
    }
    pub fn label(&self) -> String {
        match self {
            Self::Guid(g) => format!("g{g}"),
            Self::Hash(h) => format!("h{}", hex(h)),
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Base table with an optional patch table over it.
#[derive(Default)]
pub struct Strings {
    pub base: Option<TranslationTable>,
    pub patch: Option<TranslationTable>,
}

impl Strings {
    /// The string for a LAMS key, patch table first.
    pub fn get(&self, key: u32) -> Option<&str> {
        [&self.patch, &self.base]
            .into_iter()
            .flatten()
            .find_map(|t| t.get(key))
    }
    pub fn translate(&self, tag: &str) -> Option<&str> {
        [&self.patch, &self.base]
            .into_iter()
            .flatten()
            .find_map(|t| t.translate(tag))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NameSource {
    Translated,
    /// Tag present, no translation for `tag_NAME`: the tag is shown.
    TagOnly,
    /// No tag: the stored name.
    Stored,
    Unnamed,
}

/// `Slot.toString` with the layered tables.
pub fn display_name(slot: &Slot, strings: &Strings) -> (String, NameSource) {
    match slot.translation_tag.as_deref().filter(|t| !t.is_empty()) {
        Some(tag) => match strings.translate(&format!("{tag}_NAME")) {
            Some(text) => (text.to_owned(), NameSource::Translated),
            None => (tag.to_owned(), NameSource::TagOnly),
        },
        None if slot.name.is_empty() => ("Unnamed Level".to_owned(), NameSource::Unnamed),
        None => (slot.name.clone(), NameSource::Stored),
    }
}

/// `tag_DESC` (inferred suffix), else the stored description.
pub fn display_description(slot: &Slot, strings: &Strings) -> (String, bool) {
    let translated = slot
        .translation_tag
        .as_deref()
        .filter(|t| !t.is_empty())
        .and_then(|tag| strings.translate(&format!("{tag}_DESC")));
    match translated {
        Some(text) => (text.to_owned(), true),
        None => (slot.description.clone(), false),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StorySlot {
    /// Index into `Story::lists`.
    pub list: usize,
    pub index: usize,
    pub id: SlotId,
    pub name: String,
    pub name_source: NameSource,
    pub description: String,
    pub description_translated: bool,
    pub translation_tag: Option<String>,
    pub group: Option<SlotId>,
    pub initially_locked: bool,
    pub is_sub_level: Option<bool>,
    pub show_on_planet: Option<bool>,
    pub developer_level_type: Option<u32>,
    pub location: [f32; 4],
    pub root: Option<ResourceKey>,
    pub root_resolved: bool,
    pub icon: Option<ResourceKey>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoryList {
    /// Sorted MAP paths sharing this content hash.
    pub paths: Vec<String>,
    pub sha1: String,
    /// "complete", "complete_with_trailing_bytes", or the reason the list was not read.
    pub status: String,
    pub slots: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Icon {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TableState {
    Loaded,
    Missing,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableStatus {
    pub guid: u32,
    pub state: TableState,
    pub detail: String,
}

/// Measured counts; unit: slot entry in a distinct slot-list content hash.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StoryCounts {
    pub lists: usize,
    pub lists_read: usize,
    pub slots: usize,
    pub distinct_ids: usize,
    pub by_type: BTreeMap<String, usize>,
    pub by_name: BTreeMap<String, usize>,
    pub descriptions_translated: usize,
    pub with_icon: usize,
    pub icon_resolved: usize,
    pub icon_decoded: usize,
    /// Distinct icon references and how many of them decoded.
    pub distinct_icons: usize,
    pub distinct_icons_decoded: usize,
    pub with_root: usize,
    pub root_resolved: usize,
    pub initially_locked: usize,
    pub grouped: usize,
    /// Slots whose group id is not a slot in any list.
    pub group_missing: usize,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Story {
    pub lists: Vec<StoryList>,
    pub slots: Vec<StorySlot>,
    pub icons: BTreeMap<ResourceKey, Icon>,
    /// Icon references that did not decode, with the reason.
    pub icon_failures: BTreeMap<ResourceKey, String>,
    pub tables: Vec<TableStatus>,
    pub counts: StoryCounts,
}

impl Story {
    /// The slot whose id is `id` (first in list order).
    pub fn find(&self, id: SlotId) -> Option<&StorySlot> {
        self.slots.iter().find(|s| s.id == id)
    }
}

/// One slot list as read: MAP paths, content hash and the read outcome.
pub struct ListInput {
    pub paths: Vec<String>,
    pub sha1: String,
    pub result: Result<(SlotList, usize), String>,
}

/// Builds the story from read lists. `resolves` says whether a reference is in the
/// catalog; `icons`/`icon_failures` are filled by the caller (or left empty).
pub fn build_story(
    lists: Vec<ListInput>,
    strings: &Strings,
    resolves: &dyn Fn(ResourceKey) -> bool,
) -> Story {
    let mut story = Story::default();
    for (list_index, input) in lists.into_iter().enumerate() {
        let (status, count) = match input.result {
            Ok((list, trailing)) => {
                for (index, slot) in list.slots.iter().enumerate() {
                    story
                        .slots
                        .push(story_slot(list_index, index, slot, strings, resolves));
                }
                let status = if trailing == 0 {
                    "complete".to_owned()
                } else {
                    "complete_with_trailing_bytes".to_owned()
                };
                (status, list.slots.len())
            }
            Err(reason) => (reason, 0),
        };
        story.lists.push(StoryList {
            paths: input.paths,
            sha1: input.sha1,
            status,
            slots: count,
        });
    }
    story.counts = count(&story);
    story
}

fn story_slot(
    list: usize,
    index: usize,
    slot: &Slot,
    strings: &Strings,
    resolves: &dyn Fn(ResourceKey) -> bool,
) -> StorySlot {
    let (name, name_source) = display_name(slot, strings);
    let (description, description_translated) = display_description(slot, strings);
    let root = slot.root.as_ref().and_then(ResourceKey::of);
    StorySlot {
        list,
        index,
        id: slot.id,
        name,
        name_source,
        description,
        description_translated,
        translation_tag: slot.translation_tag.clone(),
        // Toolkit's SlotID default (DEVELOPER, 0) means "no group" (STRONG_INFERENCE).
        group: slot.group.filter(|g| g.slot_type != 0 || g.number != 0),
        initially_locked: slot.initially_locked,
        is_sub_level: slot.is_sub_level,
        show_on_planet: slot.show_on_planet,
        developer_level_type: slot.developer_level_type,
        location: slot.location,
        root,
        root_resolved: root.is_some_and(resolves),
        icon: slot.icon.as_ref().and_then(ResourceKey::of),
    }
}

/// Recomputes the counts (call again after icons are decoded).
pub fn count(story: &Story) -> StoryCounts {
    let mut c = StoryCounts {
        lists: story.lists.len(),
        lists_read: story
            .lists
            .iter()
            .filter(|l| l.status.starts_with("complete"))
            .count(),
        slots: story.slots.len(),
        ..Default::default()
    };
    let ids: BTreeSet<_> = story
        .slots
        .iter()
        .map(|s| (s.id.slot_type, s.id.number))
        .collect();
    c.distinct_ids = ids.len();
    let mut icons = BTreeSet::new();
    for s in &story.slots {
        *c.by_type
            .entry(s.id.type_name().unwrap_or("unknown").to_owned())
            .or_default() += 1;
        *c.by_name.entry(format!("{:?}", s.name_source)).or_default() += 1;
        c.descriptions_translated += usize::from(s.description_translated);
        if let Some(icon) = s.icon {
            icons.insert(icon);
            c.with_icon += 1;
            c.icon_decoded += usize::from(story.icons.contains_key(&icon));
            c.icon_resolved += usize::from(
                story.icons.contains_key(&icon)
                    || story
                        .icon_failures
                        .get(&icon)
                        .is_some_and(|e| !e.starts_with("Unresolved")),
            );
        }
        c.with_root += usize::from(s.root.is_some());
        c.root_resolved += usize::from(s.root_resolved);
        c.initially_locked += usize::from(s.initially_locked);
        if let Some(g) = s.group {
            c.grouped += 1;
            c.group_missing += usize::from(!ids.contains(&(g.slot_type, g.number)));
        }
    }
    c.distinct_icons = icons.len();
    c.distinct_icons_decoded = icons.iter().filter(|i| story.icons.contains_key(i)).count();
    c
}

/// Text lines for the in-app counts panel.
pub fn count_lines(story: &Story) -> Vec<String> {
    let c = &story.counts;
    let tables = story
        .tables
        .iter()
        .map(|t| format!("g{} {:?}", t.guid, t.state))
        .collect::<Vec<_>>()
        .join(", ");
    let named = c.by_name.get("Translated").copied().unwrap_or(0);
    vec![
        format!(
            "Slot lists read {}/{} (unit: distinct content hash); slots {} ({} distinct ids)",
            c.lists_read, c.lists, c.slots, c.distinct_ids
        ),
        format!(
            "Names translated {named}/{} (tag only {}, stored {}, unnamed {}); descriptions translated {} (_DESC inferred)",
            c.slots,
            c.by_name.get("TagOnly").copied().unwrap_or(0),
            c.by_name.get("Stored").copied().unwrap_or(0),
            c.by_name.get("Unnamed").copied().unwrap_or(0),
            c.descriptions_translated
        ),
        format!(
            "Icons shown {}/{} slots with an icon ({} resolved; distinct {}/{}); root levels resolved {}/{}",
            c.icon_decoded,
            c.with_icon,
            c.icon_resolved,
            c.distinct_icons_decoded,
            c.distinct_icons,
            c.root_resolved,
            c.with_root
        ),
        format!(
            "Locked at start {}; grouped {} (group not found {}); tables: {}",
            c.initially_locked,
            c.grouped,
            c.group_missing,
            if tables.is_empty() { "none".into() } else { tables }
        ),
    ]
}

fn load_table(
    source: &AttachedSource,
    catalog: &Catalog,
    guid: u32,
    control: &LoadControl,
) -> (Option<TranslationTable>, TableStatus) {
    let status = |state, detail: String| TableStatus {
        guid,
        state,
        detail,
    };
    let rows: Vec<_> = catalog
        .by_guid
        .get(&format!("{guid:08x}"))
        .into_iter()
        .flatten()
        .filter_map(|&i| catalog.snapshot.resources.get(i))
        .collect();
    let Some(row) = rows
        .iter()
        .find(|r| r.map_row.path.to_ascii_lowercase().ends_with(".trans"))
    else {
        let detail = if rows.is_empty() {
            "no MAP row".to_owned()
        } else {
            "MAP row is not a .trans path".to_owned()
        };
        return (None, status(TableState::Missing, detail));
    };
    let path = row.map_row.path.clone();
    let loaded = crate::resolve(catalog, Some(guid), None)
        .and_then(|key| read_resource(source, &key, control, MAX_TRANSLATION_BYTES))
        .and_then(|bytes| {
            parse_translation_table(&bytes, TranslationLimits::default()).map_err(|e| e.to_string())
        });
    match loaded {
        Ok(table) => {
            let detail = format!("{path}: {} keys", table.distinct_keys());
            (Some(table), status(TableState::Loaded, detail))
        }
        Err(e) => (None, status(TableState::Failed, format!("{path}: {e}"))),
    }
}

pub fn resolve_key(
    catalog: &Catalog,
    key: ResourceKey,
) -> Result<lbp_catalog::attachment::LocationKey, String> {
    match key {
        ResourceKey::Guid(g) => crate::resolve(catalog, Some(g), None),
        ResourceKey::Hash(h) => crate::resolve(catalog, None, Some(&hex(&h))),
    }
}

/// Decodes one icon texture (2D top mip, RGBA8).
pub fn decode_icon(
    source: &AttachedSource,
    catalog: &Catalog,
    key: ResourceKey,
    control: &LoadControl,
) -> Result<Icon, String> {
    let location = resolve_key(catalog, key)?;
    let texture = lbp_texture::decode_reader(
        location.size,
        |offset, count| {
            source
                .read_hex_checked(&location, offset, count, control)
                .map(|page| page.bytes)
                .map_err(|_| lbp_texture::Error::Invalid("source read failed"))
        },
        || control.check().is_err(),
    )
    .map_err(|e| format!("Unsupported: icon texture: {e}"))?;
    Ok(Icon {
        width: texture.header.width,
        height: texture.header.height,
        rgba: texture.rgba,
    })
}

/// The English base table with the patch table over it, and how each loaded.
pub fn load_strings(
    source: &AttachedSource,
    catalog: &Catalog,
    control: &LoadControl,
) -> (Strings, Vec<TableStatus>) {
    let mut strings = Strings::default();
    let mut tables = Vec::new();
    for (guid, patch) in [
        (BASE_TRANSLATION_GUID, false),
        (PATCH_TRANSLATION_GUID, true),
    ] {
        let (table, status) = load_table(source, catalog, guid, control);
        if patch {
            strings.patch = table;
        } else {
            strings.base = table;
        }
        tables.push(status);
    }
    (strings, tables)
}

/// Reads every magic-confirmed slot list, the English tables and the slot icons.
/// D328: the story slot list `gamedata/data/lbp2story_slots.slt` (GUID 000162ec), the
/// list the pod's Story search makes its results from.
pub fn read_story_slot_list(
    source: &AttachedSource,
    catalog: &Catalog,
    control: &LoadControl,
) -> Result<SlotList, String> {
    let key = crate::resolve(catalog, Some(0x162ec), None)?;
    let bytes = read_resource(source, &key, control, 16 << 20)?;
    let report = slots::read(&bytes, Limits::default(), &|| false).map_err(|e| e.to_string())?;
    match report.result {
        Ok(UiData::SlotList(list)) => Ok(list),
        Ok(_) => Err("lbp2story_slots.slt is not a slot list".into()),
        Err(e) => Err(format!("lbp2story_slots.slt: {e:?}")),
    }
}

pub fn load_story(
    source: &AttachedSource,
    catalog: &Catalog,
    control: &LoadControl,
) -> Result<Story, String> {
    let (strings, tables) = load_strings(source, catalog, control);
    // Content hash -> sorted paths.
    let mut by_hash: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for r in &catalog.snapshot.resources {
        if r.display_type() == "slot_list" {
            by_hash
                .entry(r.map_row.sha1.to_ascii_lowercase())
                .or_default()
                .insert(r.map_row.path.clone());
        }
    }
    let mut lists = Vec::new();
    for (sha1, paths) in by_hash {
        control.check()?;
        let result = crate::resolve(catalog, None, Some(&sha1))
            .and_then(|key| read_resource(source, &key, control, slots::MAX_INPUT))
            .and_then(|bytes| {
                let report = slots::read(&bytes, Limits::default(), &|| control.check().is_err())
                    .map_err(|e| e.to_string())?;
                match report.result {
                    Ok(UiData::SlotList(list)) => Ok((list, report.checks.trailing_bytes)),
                    Ok(_) => Err("Invalid: not a slot list".to_owned()),
                    Err(stop) => Err(format!("stopped: {}", stop.label())),
                }
            });
        lists.push(ListInput {
            paths: paths.into_iter().collect(),
            sha1,
            result,
        });
    }
    let resolves = |key| resolve_key(catalog, key).is_ok();
    let mut story = build_story(lists, &strings, &resolves);
    story.tables = tables;
    let wanted: BTreeSet<ResourceKey> = story.slots.iter().filter_map(|s| s.icon).collect();
    let mut bytes = 0usize;
    for key in wanted {
        control.check()?;
        if story.icons.len() >= MAX_ICONS {
            story
                .icon_failures
                .insert(key, "Budget: icon count".to_owned());
            continue;
        }
        match decode_icon(source, catalog, key, control) {
            Ok(icon) if bytes + icon.rgba.len() <= MAX_ICON_BYTES => {
                bytes += icon.rgba.len();
                story.icons.insert(key, icon);
            }
            Ok(_) => {
                story
                    .icon_failures
                    .insert(key, "Budget: icon bytes".to_owned());
            }
            Err(e) => {
                story.icon_failures.insert(key, e);
            }
        }
    }
    story.counts = count(&story);
    Ok(story)
}
