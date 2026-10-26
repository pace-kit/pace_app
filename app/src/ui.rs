use crate::{
    browser,
    editor::{Editor, Selection},
};
use dioxus::prelude::*;

#[component]
pub fn App() -> Element {
    let mut notice = use_signal(String::new);
    let mut editor = use_signal(|| {
        let mut editor = Editor::default();
        match browser::load() {
            Ok(Some(text)) => match editor.open(&text) {
                Ok(()) => notice.set("Recovered your last browser draft.".into()),
                Err(e) => notice.set(format!("Recovery was not opened: {e}")),
            },
            Err(e) => notice.set(e),
            _ => {}
        }
        editor
    });
    use_effect(move || browser::set_dirty(editor.read().dirty));
    let issues = editor.read().project.validate();
    let has_issues = !issues.is_empty();
    let mut session = use_signal(|| None::<crate::rehearsal::Rehearsal>);
    rsx! {
        style { {include_str!("editor.css")} }
        div { class: "app",
            header { class: "topbar",
                div { class: "brand", "P" }
                div { class: "brand-copy", strong { "PACE" } span { "Adventure editor" } }
                div { class: "project-title", "{editor.read().project.title}" if editor.read().dirty { span { title: "Changes not yet saved to file", " •" } } }
                button { onclick: move |_| { if !editor.read().dirty || browser::confirm("Discard changes that have not been saved to a project file?") { editor.set(Editor::default()); } }, "New project" }
                label { class: "button", "Open project"
                    input { r#type: "file", accept: ".json", "aria-label": "Open project", onchange: move |event: Event<FormData>| async move {
                        if editor.read().dirty && !browser::confirm("Replace this project? Save a file first to keep your changes.") { return; }
                        let document_id = editor.read().document_id;
                        let original = editor.read().project.clone();
                        if let Some(files) = event.files() {
                            if let Some(name) = files.files().first() {
                                match files.read_file_to_string(name).await {
                                    Some(_) if editor.read().document_id != document_id || editor.read().project != original => notice.set("Open cancelled because the project changed while the file was being read. Try opening it again.".into()),
                                    Some(text) => match editor.write().open(&text) { Ok(()) => notice.set("Project opened.".into()), Err(e) => notice.set(e) },
                                    None => notice.set("Could not read project file.".into()),
                                }
                            }
                        }
                    } }
                }
                button { class: "primary", onclick: move |_| {
                    let result = editor.read().project.to_json().and_then(|text| { browser::download(&text)?; Ok(text) });
                    match result {
                        Ok(text) => match browser::recover(&text) {
                            Ok(()) => { editor.write().dirty = false; notice.set("Project download requested and browser recovery saved.".into()); },
                            Err(e) => notice.set(format!("Project download requested. {e}")),
                        },
                        Err(e) => notice.set(e),
                    }
                }, "Save project" }
            }
            div { class: "dependency-banner", "Editor draft v1 · Your project files include all imported images." }
            crate::sharing::SharingPanel { editor }
            div { class: "workspace",
                aside { class: "sidebar",
                    button { class: "project-link", onclick: move |_| editor.write().selection = Selection::Project, "Project settings" }
                    div { class: "section-heading", h2 { "ROOMS" } button { "aria-label": "Add room", onclick: move |_| {
                        let mut id = String::new(); editor.write().edit(|p| id = p.add_room()); editor.write().selection = Selection::Room(id);
                    }, "+" } }
                    for room in editor.read().project.rooms.clone() {
                        button { class: if editor.read().selected_room() == Some(room.id.as_str()) { "tree-row active" } else { "tree-row" }, onclick: move |_| editor.write().selection = Selection::Room(room.id.clone()), "{room.name}" }
                    }
                    div { class: "section-heading", h2 { "INVENTORY" } button { "aria-label": "Add item", onclick: move |_| {
                        let mut id = String::new(); editor.write().edit(|p| id = p.add_item()); editor.write().selection = Selection::Item(id);
                    }, "+" } }
                    for item in editor.read().project.items.clone() {
                        button { class: if editor.read().selection == Selection::Item(item.id.clone()) { "tree-row active" } else { "tree-row" }, onclick: move |_| editor.write().selection = Selection::Item(item.id.clone()), "{item.name}" }
                    }
                    div { class: "section-heading", h2 { "DIALOGUE" } button { "aria-label": "Add dialogue", onclick: move |_| {
                        let mut id = String::new(); editor.write().edit(|p| id = p.add_dialogue()); editor.write().selection = Selection::Dialogue(id);
                    }, "+" } }
                    for dialogue in editor.read().project.dialogues.clone() {
                        button { class: if editor.read().selection == Selection::Dialogue(dialogue.id.clone()) { "tree-row active" } else { "tree-row" }, onclick: move |_| editor.write().selection = Selection::Dialogue(dialogue.id.clone()), "{dialogue.name}" }
                    }
                    div { class: "sidebar-foot", "Build a world, one room at a time." }
                }
                main { class: "main-panel",
                    div { class: "toolbar",
                        span { class: "eyebrow", "YOUR ADVENTURE" }
                        button { disabled: !editor.read().can_undo(), onclick: move |_| editor.write().undo(), "Undo" }
                        button { disabled: !editor.read().can_redo(), onclick: move |_| editor.write().redo(), "Redo" }
                        button { disabled: has_issues, onclick: move |_| {
                            match crate::rehearsal::Rehearsal::new(&editor.read().project) { Ok(state) => session.set(Some(state)), Err(e) => notice.set(e) }
                        }, "Rehearse draft" }
                        button { disabled: true, title: "Draft conversion and a browser build of the PACE core runtime are not integrated yet.", "Core preview" }
                        button { disabled: true, title: "Game export requires a pinned PACE core runtime.", "Export game" }
                    }
                    if matches!(editor.read().selection, Selection::Item(_) | Selection::Dialogue(_)) { crate::story::StoryWorkspace { editor } } else {
                    crate::rooms::RoomWorkspace { key: "{editor.read().selected_room().unwrap_or_default()}", editor, notice }
                    }
                }
                aside { class: "inspector",
                    if editor.read().selected_room().is_some() { crate::rooms::RoomInspector { editor } } else {
                    h2 { "Project" }
                    label { "Project title" input { value: "{editor.read().project.title}", oninput: move |e| editor.write().edit(|p| p.title = e.value()) } }
                    label { "Starting room" select { value: editor.read().project.start_room.clone().unwrap_or_default(), onchange: move |e| editor.write().edit(|p| p.start_room = if e.value().is_empty() { None } else { Some(e.value()) }), option { value: "", selected: editor.read().project.start_room.is_none(), "Choose a room" } for room in editor.read().project.rooms.clone() { option { value: "{room.id}", selected: editor.read().project.start_room.as_deref() == Some(room.id.as_str()), "{room.name}" } } } }
                    }
                    h3 { "Validation · {issues.len()}" }
                    if issues.is_empty() { p { class: "success", "Ready for draft authoring." } }
                    for issue in issues {
                        button { class: "issue", onclick: move |_| editor.write().focus(issue.object_id.as_deref()), strong { {format!("{} · {}", issue.object_id.as_deref().unwrap_or("Project"), issue.field)} } span { "{issue.message}" } }
                    }
                }
            }
            footer { role: "status", "{notice}" }
            if session.read().is_some() { crate::preview::DraftPreview { session } }
        }
    }
}
