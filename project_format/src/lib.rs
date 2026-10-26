//! Editor-owned draft contract. This is deliberately NOT named the PACE core
//! runtime scene format. Embedded backgrounds and editable drafts are preserved.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

pub const FORMAT: &str = "pace-editor-draft";
pub const VERSION: u32 = 1;
pub const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_PROJECT_BYTES: usize = 64 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub format: String,
    pub version: u32,
    pub title: String,
    pub start_room: Option<String>,
    pub rooms: Vec<Room>,
    pub items: Vec<Item>,
    pub dialogues: Vec<Dialogue>,
    next_id: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub background: Option<Background>,
    pub hotspots: Vec<Hotspot>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Background {
    pub name: String,
    pub data_url: Arc<str>,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    /// Coordinates are normalized to the original image, independent of zoom.
    pub fn from_drag(start: (f64, f64), end: (f64, f64)) -> Option<Self> {
        let (ax, ay) = (start.0.clamp(0., 1.), start.1.clamp(0., 1.));
        let (bx, by) = (end.0.clamp(0., 1.), end.1.clamp(0., 1.));
        let rect = Self {
            x: ax.min(bx),
            y: ay.min(by),
            width: (ax - bx).abs(),
            height: (ay - by).abs(),
        };
        (rect.valid() && rect.width >= 0.005 && rect.height >= 0.005).then_some(rect)
    }

    pub fn valid(&self) -> bool {
        [self.x, self.y, self.width, self.height]
            .iter()
            .all(|n| n.is_finite())
            && self.x >= 0.
            && self.y >= 0.
            && self.width > 0.
            && self.height > 0.
            && self.x + self.width <= 1.000001
            && self.y + self.height <= 1.000001
    }

    pub fn translated(self, dx: f64, dy: f64) -> Self {
        Self {
            x: (self.x + dx).clamp(0., (1. - self.width).max(0.)),
            y: (self.y + dy).clamp(0., (1. - self.height).max(0.)),
            ..self
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Hotspot {
    pub id: String,
    pub name: String,
    pub rect: Rect,
    pub action: Action,
    pub requires_item: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Look { text: String },
    GoTo { room_id: String },
    Take { item_id: String },
    Talk { dialogue_id: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dialogue {
    pub id: String,
    pub name: String,
    pub speaker: String,
    pub text: String,
    pub choices: Vec<Choice>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Choice {
    pub id: String,
    pub text: String,
    pub next: Option<String>,
    pub give_item: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub object_id: Option<String>,
    pub field: String,
    pub message: String,
}

impl Default for Project {
    fn default() -> Self {
        Self {
            format: FORMAT.into(),
            version: VERSION,
            title: "Untitled adventure".into(),
            start_room: None,
            rooms: vec![],
            items: vec![],
            dialogues: vec![],
            next_id: 1,
        }
    }
}

impl Project {
    pub fn to_json(&self) -> Result<String, String> {
        if self.rooms.iter().flat_map(|r| &r.hotspots).any(|h| {
            [h.rect.x, h.rect.y, h.rect.width, h.rect.height]
                .iter()
                .any(|n| !n.is_finite())
        }) {
            return Err(
                "Cannot save a hotspot with non-finite coordinates. Correct its bounds first."
                    .into(),
            );
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        if text.len() > MAX_PROJECT_BYTES {
            return Err("Project exceeds the 64 MiB file limit. Reduce background sizes.".into());
        }
        Ok(text)
    }

    pub fn from_json(text: &str) -> Result<Self, String> {
        if text.len() > MAX_PROJECT_BYTES {
            return Err("Project exceeds the 64 MiB file limit.".into());
        }
        let value: serde_json::Value =
            serde_json::from_str(text).map_err(|e| format!("Invalid project JSON: {e}"))?;
        if value.get("format").and_then(|v| v.as_str()) != Some(FORMAT) {
            return Err(
                "This is not a PACE editor draft. Core-format import awaits the upstream contract."
                    .into(),
            );
        }
        if value.get("version").and_then(|v| v.as_u64()) != Some(VERSION as u64) {
            return Err("Unsupported draft version. Open it with the editor that created it; no data was replaced.".into());
        }
        // Strict decoding prevents silent loss of fields from a newer writer.
        let project: Self = serde_json::from_str(text)
            .map_err(|e| format!("Unsupported or malformed project: {e}"))?;
        // Invalid references are editable, but ambiguous IDs cannot safely be edited.
        let mut ids = HashSet::new();
        for id in project.ids() {
            if id.trim().is_empty() || !ids.insert(id) {
                return Err(format!(
                    "Empty or duplicate object ID: {id:?}. No data was replaced."
                ));
            }
        }
        Ok(project)
    }

    fn ids(&self) -> Vec<&str> {
        self.rooms
            .iter()
            .flat_map(|r| {
                std::iter::once(r.id.as_str()).chain(r.hotspots.iter().map(|h| h.id.as_str()))
            })
            .chain(self.items.iter().map(|i| i.id.as_str()))
            .chain(self.dialogues.iter().flat_map(|d| {
                std::iter::once(d.id.as_str()).chain(d.choices.iter().map(|c| c.id.as_str()))
            }))
            .collect()
    }

    pub fn allocate_id(&mut self, kind: &str) -> String {
        loop {
            let id = format!("{kind}-{}", self.next_id);
            self.next_id = self.next_id.wrapping_add(1);
            if !self.ids().contains(&id.as_str()) {
                return id;
            }
        }
    }

    pub fn add_room(&mut self) -> String {
        let id = self.allocate_id("room");
        self.rooms.push(Room {
            id: id.clone(),
            name: format!("Room {}", self.rooms.len() + 1),
            background: None,
            hotspots: vec![],
        });
        if self.start_room.is_none() {
            self.start_room = Some(id.clone());
        }
        id
    }

    pub fn room(&self, id: &str) -> Option<&Room> {
        self.rooms.iter().find(|r| r.id == id)
    }
    pub fn room_mut(&mut self, id: &str) -> Option<&mut Room> {
        self.rooms.iter_mut().find(|r| r.id == id)
    }

    pub fn add_hotspot(&mut self, room_id: &str, rect: Rect) -> Option<String> {
        if !rect.valid() || self.room(room_id)?.background.is_none() {
            return None;
        }
        let id = self.allocate_id("hotspot");
        let room = self.room_mut(room_id)?;
        room.hotspots.push(Hotspot {
            id: id.clone(),
            name: format!("Hotspot {}", room.hotspots.len() + 1),
            rect,
            action: Action::Look {
                text: "Something catches your eye.".into(),
            },
            requires_item: None,
        });
        Some(id)
    }

    pub fn add_item(&mut self) -> String {
        let id = self.allocate_id("item");
        self.items.push(Item {
            id: id.clone(),
            name: "New item".into(),
            description: String::new(),
        });
        id
    }

    pub fn add_dialogue(&mut self) -> String {
        let id = self.allocate_id("dialogue");
        self.dialogues.push(Dialogue {
            id: id.clone(),
            name: "New dialogue".into(),
            speaker: "Narrator".into(),
            text: "Hello, traveler.".into(),
            choices: vec![],
        });
        id
    }

    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = vec![];
        let mut push = |id: Option<&str>, field: &str, message: String| {
            issues.push(Issue {
                object_id: id.map(str::to_owned),
                field: field.into(),
                message,
            })
        };
        if self.title.trim().is_empty() {
            push(None, "title", "Give the project a title.".into());
        }
        if self
            .start_room
            .as_deref()
            .is_none_or(|id| self.room(id).is_none())
        {
            push(
                None,
                "start_room",
                "Choose an existing starting room.".into(),
            );
        }
        let has_item = |id: &str| self.items.iter().any(|i| i.id == id);
        let has_dialogue = |id: &str| self.dialogues.iter().any(|d| d.id == id);
        for r in &self.rooms {
            if r.name.trim().is_empty() {
                push(Some(&r.id), "name", "Room name is empty.".into());
            }
            match &r.background {
                None => push(
                    Some(&r.id),
                    "background",
                    "Import a background for this room.".into(),
                ),
                Some(b) => {
                    if let Err(e) = b.validate() {
                        push(Some(&r.id), "background", e);
                    }
                }
            }
            for h in &r.hotspots {
                if h.name.trim().is_empty() {
                    push(Some(&h.id), "name", "Hotspot name is empty.".into());
                }
                if !h.rect.valid() {
                    push(
                        Some(&h.id),
                        "rect",
                        "Hotspot must have a positive size and fit inside the background.".into(),
                    );
                }
                if let Some(id) = &h.requires_item {
                    if !has_item(id) {
                        push(
                            Some(&h.id),
                            "requires_item",
                            format!("Required item {id:?} does not exist."),
                        );
                    }
                }
                match &h.action {
                    Action::GoTo { room_id } if self.room(room_id).is_none() => push(
                        Some(&h.id),
                        "action.room_id",
                        format!("Linked room {room_id:?} does not exist."),
                    ),
                    Action::Take { item_id } if !has_item(item_id) => push(
                        Some(&h.id),
                        "action.item_id",
                        format!("Item {item_id:?} does not exist."),
                    ),
                    Action::Talk { dialogue_id } if !has_dialogue(dialogue_id) => push(
                        Some(&h.id),
                        "action.dialogue_id",
                        format!("Dialogue {dialogue_id:?} does not exist."),
                    ),
                    Action::Look { text } if text.trim().is_empty() => {
                        push(Some(&h.id), "action.text", "Enter inspection text.".into())
                    }
                    _ => {}
                }
            }
        }
        for item in &self.items {
            if item.name.trim().is_empty() {
                push(Some(&item.id), "name", "Item name is empty.".into());
            }
        }
        for d in &self.dialogues {
            if d.name.trim().is_empty() {
                push(Some(&d.id), "name", "Dialogue name is empty.".into());
            }
            if d.text.trim().is_empty() {
                push(Some(&d.id), "text", "Enter dialogue text.".into());
            }
            for c in &d.choices {
                if c.text.trim().is_empty() {
                    push(Some(&c.id), "text", "Enter choice text.".into());
                }
                if let Some(id) = &c.next {
                    if !has_dialogue(id) {
                        push(
                            Some(&c.id),
                            "next",
                            format!("Next dialogue {id:?} does not exist."),
                        );
                    }
                }
                if let Some(id) = &c.give_item {
                    if !has_item(id) {
                        push(
                            Some(&c.id),
                            "give_item",
                            format!("Reward item {id:?} does not exist."),
                        );
                    }
                }
            }
        }
        issues
    }
}

impl Background {
    /// Only embedded raster data is rendered; malformed imports cannot load a
    /// remote URL or an active document through the image element.
    pub fn display_url(&self) -> &str {
        if [
            "data:image/png;base64,",
            "data:image/jpeg;base64,",
            "data:image/webp;base64,",
        ]
        .iter()
        .any(|prefix| self.data_url.starts_with(prefix))
        {
            &self.data_url
        } else {
            ""
        }
    }

    pub fn import(name: String, bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err("Background exceeds 8 MiB. Resize it before importing.".into());
        }
        let mime = if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
            "image/png"
        } else if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
            "image/jpeg"
        } else if bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP") {
            "image/webp"
        } else {
            return Err("Use a PNG, JPEG, or WebP image.".into());
        };
        let size = imagesize::blob_size(bytes)
            .map_err(|_| "Cannot read this image. It may be corrupt.".to_string())?;
        if size.width == 0 || size.height == 0 || size.width > 16384 || size.height > 16384 {
            return Err("Image dimensions must be between 1 and 16384 pixels.".into());
        }
        Ok(Self {
            name,
            data_url: format!("data:{mime};base64,{}", STANDARD.encode(bytes)).into(),
            width: size.width as u32,
            height: size.height as u32,
        })
    }

    fn validate(&self) -> Result<(), String> {
        let (header, data) = self
            .data_url
            .split_once(',')
            .ok_or("Invalid embedded background.")?;
        if data.len() > MAX_IMAGE_BYTES * 4 / 3 + 4 {
            return Err("Embedded background exceeds 8 MiB.".into());
        }
        let bytes = STANDARD
            .decode(data)
            .map_err(|_| "Invalid background encoding.")?;
        let parsed = Self::import(self.name.clone(), &bytes)?;
        if parsed.data_url.split_once(',').map(|p| p.0) != Some(header)
            || parsed.width != self.width
            || parsed.height != self.height
        {
            return Err("Background type or dimensions do not match its image data.".into());
        }
        Ok(())
    }
}
