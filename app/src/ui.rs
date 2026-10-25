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
    let issues = editor.read().project.validate();
    rsx! {
        style { {include_str!("editor.css")} }
        div { class: "app",
            header { class: "topbar",
                div { class: "brand", "P" }
                div { class: "brand-copy", strong { "PACE" } span { "Adventure editor" } }
                div { class: "project-title", "{editor.read().project.title}" }
                button { onclick: move |_| { if !editor.read().dirty || browser::confirm("Discard changes that have not been saved to a project file?") { editor.set(Editor::default()); } }, "New project" }
                label { class: "button", "Open project"
                    input { r#type: "file", accept: ".json", "aria-label": "Open project", onchange: move |event: Event<FormData>| async move {
                        if editor.read().dirty && !browser::confirm("Replace this project? Save a file first to keep your changes.") { return; }
                        if let Some(files) = event.files() {
                            if let Some(name) = files.files().first() {
                                match files.read_file_to_string(name).await {
                                    Some(text) => match editor.write().open(&text) { Ok(()) => notice.set("Project opened.".into()), Err(e) => notice.set(e) },
                                    None => notice.set("Could not read project file.".into()),
                                }
                            }
                        }
                    } }
                }
                button { class: "primary", onclick: move |_| {
                    let result = editor.read().project.to_json().and_then(|text| { browser::download(&text)?; Ok(text) });
                    match result { Ok(text) => { editor.write().dirty = false; notice.set(match browser::recover(&text) { Ok(()) => "Project file downloaded and browser recovery saved.".into(), Err(e) => format!("Project file downloaded. {e}") }); }, Err(e) => notice.set(e) }
                }, "Save project" }
            }
            div { class: "dependency-banner", "Draft format v1 · PACE core integration pending. Your project files include all imported images." }
            div { class: "workspace",
                aside { class: "sidebar",
                    button { class: "project-link", onclick: move |_| editor.write().selection = Selection::Project, "Project settings" }
                    div { class: "section-heading", h2 { "ROOMS" } button { "aria-label": "Add room", onclick: move |_| {
                        let mut id = String::new(); editor.write().edit(|p| id = p.add_room()); editor.write().selection = Selection::Room(id);
                    }, "+" } }
                    for room in editor.read().project.rooms.clone() {
                        button { class: if editor.read().selected_room() == Some(room.id.as_str()) { "tree-row active" } else { "tree-row" }, onclick: move |_| editor.write().selection = Selection::Room(room.id.clone()), "{room.name}" }
                    }
                    div { class: "sidebar-foot", "Build a world, one room at a time." }
                }
                main { class: "main-panel",
                    div { class: "toolbar",
                        span { class: "eyebrow", "YOUR ADVENTURE" }
                        button { disabled: !editor.read().can_undo(), onclick: move |_| editor.write().undo(), "Undo" }
                        button { disabled: !editor.read().can_redo(), onclick: move |_| editor.write().redo(), "Redo" }
                    }
                    div { class: "empty-state", span { class: "empty-icon", "◇" } h1 { "Every adventure starts somewhere." } p { "Add a room, import its background, and bring your world to life." } }
                }
                aside { class: "inspector",
                    h2 { "Project" }
                    label { "Project title" input { value: "{editor.read().project.title}", oninput: move |e| editor.write().edit(|p| p.title = e.value()) } }
                    label { "Starting room" select { value: editor.read().project.start_room.clone().unwrap_or_default(), onchange: move |e| editor.write().edit(|p| p.start_room = if e.value().is_empty() { None } else { Some(e.value()) }), option { value: "", "Choose a room" } for room in editor.read().project.rooms.clone() { option { value: "{room.id}", "{room.name}" } } } }
                    h3 { "Validation · {issues.len()}" }
                    if issues.is_empty() { p { class: "success", "Ready for draft authoring." } }
                    for issue in issues {
                        button { class: "issue", onclick: move |_| editor.write().focus(issue.object_id.as_deref()), strong { "{issue.field}" } span { "{issue.message}" } }
                    }
                }
            }
            footer { role: "status", "{notice}" }
        }
    }
}
