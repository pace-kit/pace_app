//! An editor-only interaction rehearsal, not the unavailable PACE runtime.
//! It runs on an isolated snapshot and must never be used as game export.
use crate::model::{Action, Project};
use std::collections::BTreeSet;

#[derive(Clone)]
pub struct Rehearsal {
    pub project: Project,
    pub room_id: String,
    pub inventory: BTreeSet<String>,
    pub dialogue_id: Option<String>,
    pub message: String,
}
impl Rehearsal {
    pub fn new(project: &Project) -> Result<Self, String> {
        let issues = project.validate();
        if !issues.is_empty() {
            return Err(format!(
                "Resolve {} validation error(s) before rehearsing.",
                issues.len()
            ));
        }
        Ok(Self {
            project: project.clone(),
            room_id: project.start_room.clone().ok_or("Choose a starting room")?,
            inventory: BTreeSet::new(),
            dialogue_id: None,
            message: "Click a hotspot to try its action.".into(),
        })
    }
    pub fn activate(&mut self, hotspot_id: &str) -> Result<(), String> {
        if self.dialogue_id.is_some() {
            return Err("Finish the conversation first.".into());
        }
        let h = self
            .project
            .room(&self.room_id)
            .and_then(|r| r.hotspots.iter().find(|h| h.id == hotspot_id))
            .ok_or("Hotspot is not in the current room")?
            .clone();
        if let Some(id) = &h.requires_item {
            if !self.inventory.contains(id) {
                let name = self
                    .project
                    .items
                    .iter()
                    .find(|i| &i.id == id)
                    .map(|i| i.name.as_str())
                    .unwrap_or(id);
                self.message = format!("You need {name} first.");
                return Ok(());
            }
        }
        match h.action {
            Action::Look { text } => self.message = text,
            Action::GoTo { room_id } => {
                if self.project.room(&room_id).is_none() {
                    return Err("Destination room is missing".into());
                }
                self.room_id = room_id;
                self.message = "You enter another room.".into();
            }
            Action::Take { item_id } => {
                let item = self
                    .project
                    .items
                    .iter()
                    .find(|i| i.id == item_id)
                    .ok_or("Item is missing")?;
                self.message = if self.inventory.insert(item_id) {
                    format!("Picked up {}. {}", item.name, item.description)
                } else {
                    format!("You already have {}.", item.name)
                };
            }
            Action::Talk { dialogue_id } => {
                if !self.project.dialogues.iter().any(|d| d.id == dialogue_id) {
                    return Err("Dialogue is missing".into());
                }
                self.dialogue_id = Some(dialogue_id);
                self.message.clear();
            }
        }
        Ok(())
    }
    pub fn choose(&mut self, choice_id: &str) -> Result<(), String> {
        let choice = self
            .project
            .dialogues
            .iter()
            .find(|d| Some(&d.id) == self.dialogue_id.as_ref())
            .and_then(|d| d.choices.iter().find(|c| c.id == choice_id))
            .ok_or("Choice is not in the active dialogue")?
            .clone();
        if let Some(item_id) = choice.give_item {
            self.inventory.insert(item_id);
        }
        self.dialogue_id = choice.next;
        self.message = if self.dialogue_id.is_none() {
            "The conversation ends.".into()
        } else {
            String::new()
        };
        Ok(())
    }
    pub fn continue_dialogue(&mut self) -> Result<(), String> {
        let d = self
            .project
            .dialogues
            .iter()
            .find(|d| Some(&d.id) == self.dialogue_id.as_ref())
            .ok_or("No active conversation")?;
        if !d.choices.is_empty() {
            return Err("Choose a response first".into());
        }
        self.dialogue_id = None;
        self.message = "The conversation ends.".into();
        Ok(())
    }
}
