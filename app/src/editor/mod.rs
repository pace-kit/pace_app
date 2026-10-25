use crate::model::Project;
use std::sync::atomic::{AtomicUsize, Ordering};
static NEXT_DOCUMENT: AtomicUsize = AtomicUsize::new(1);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selection {
    Project,
    Room(String),
    Hotspot(String, String),
    Item(String),
    Dialogue(String),
}

#[derive(Clone)]
pub struct Editor {
    pub document_id: usize,
    pub project: Project,
    pub selection: Selection,
    pub dirty: bool,
    undo: Vec<Project>,
    redo: Vec<Project>,
}

impl Default for Editor {
    fn default() -> Self {
        Self::new(Project::default())
    }
}

impl Editor {
    pub fn new(project: Project) -> Self {
        let selection = project
            .rooms
            .first()
            .map(|r| Selection::Room(r.id.clone()))
            .unwrap_or(Selection::Project);
        Self {
            document_id: NEXT_DOCUMENT.fetch_add(1, Ordering::Relaxed),
            project,
            selection,
            dirty: false,
            undo: vec![],
            redo: vec![],
        }
    }
    pub fn edit(&mut self, change: impl FnOnce(&mut Project)) {
        let before = self.project.clone();
        change(&mut self.project);
        if before != self.project {
            self.undo.push(before);
            if self.undo.len() > 50 {
                self.undo.remove(0);
            }
            self.redo.clear();
            self.dirty = true;
        }
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo(&mut self) {
        if let Some(previous) = self.undo.pop() {
            self.redo
                .push(std::mem::replace(&mut self.project, previous));
            self.reconcile_selection();
            self.dirty = true;
        }
    }
    pub fn redo(&mut self) {
        if let Some(next) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.project, next));
            self.reconcile_selection();
            self.dirty = true;
        }
    }
    pub fn open(&mut self, text: &str) -> Result<(), String> {
        let project = Project::from_json(text)?;
        *self = Self::new(project);
        Ok(())
    }
    pub fn selected_room(&self) -> Option<&str> {
        match &self.selection {
            Selection::Room(id) | Selection::Hotspot(id, _) => Some(id),
            _ => None,
        }
    }
    pub fn focus(&mut self, id: Option<&str>) {
        self.selection = Selection::Project;
        if let Some(id) = id {
            for room in &self.project.rooms {
                if room.id == id {
                    self.selection = Selection::Room(id.into());
                    return;
                }
                if room.hotspots.iter().any(|h| h.id == id) {
                    self.selection = Selection::Hotspot(room.id.clone(), id.into());
                    return;
                }
            }
            if self.project.items.iter().any(|i| i.id == id) {
                self.selection = Selection::Item(id.into());
            }
            if let Some(d) = self
                .project
                .dialogues
                .iter()
                .find(|d| d.id == id || d.choices.iter().any(|c| c.id == id))
            {
                self.selection = Selection::Dialogue(d.id.clone());
            }
        }
    }
    pub fn delete_selection(&mut self) {
        let selection = self.selection.clone();
        self.edit(|p| match &selection {
            Selection::Room(id) => p.rooms.retain(|r| &r.id != id),
            Selection::Hotspot(room_id, id) => {
                if let Some(r) = p.room_mut(room_id) {
                    r.hotspots.retain(|h| &h.id != id);
                }
            }
            Selection::Item(id) => p.items.retain(|i| &i.id != id),
            Selection::Dialogue(id) => p.dialogues.retain(|d| &d.id != id),
            Selection::Project => {}
        });
        // Keep references to deleted objects so validation exposes each affected
        // object instead of silently changing the author's intended story.
        self.reconcile_selection();
    }
    fn reconcile_selection(&mut self) {
        let selection = self.selection.clone();
        let id = match &selection {
            Selection::Project => None,
            Selection::Room(id)
            | Selection::Item(id)
            | Selection::Dialogue(id)
            | Selection::Hotspot(_, id) => Some(id.as_str()),
        };
        self.focus(id);
    }
}
