use crate::rehearsal::Rehearsal;
use dioxus::prelude::*;

#[component]
pub fn DraftPreview(mut session: Signal<Option<Rehearsal>>) -> Element {
    use_effect(crate::browser::open_rehearsal);
    let state = session.read().clone()?;
    let room = state.project.room(&state.room_id)?;
    let dialogue = state
        .dialogue_id
        .as_ref()
        .and_then(|id| state.project.dialogues.iter().find(|d| &d.id == id));
    rsx! {
        dialog { id: "draft-rehearsal", class: "preview-overlay", role: "dialog", "aria-modal": "true", "aria-label": "Draft rehearsal",
            div { class: "preview",
                div { class: "preview-head", h2 { "{room.name}" }
                    button { onclick: move |_| { let project = session.read().as_ref().map(|s| s.project.clone()); if let Some(p) = project { session.set(Rehearsal::new(&p).ok()); } }, "Restart" }
                    button { id: "close-rehearsal", autofocus: true, onclick: move |_| session.set(None), "Close rehearsal" }
                }
                p { class: "preview-note", "Editor rehearsal only · PACE runtime integration is pending. This does not verify runtime compatibility." }
                if let Some(bg) = &room.background {
                    div { class: "stage", style: "aspect-ratio:{bg.width}/{bg.height}",
                        img { src: bg.display_url(), alt: "{room.name} background" }
                        for hotspot in room.hotspots.clone() {
                            button { class: "hotspot", disabled: dialogue.is_some(), style: format!("left:{}%;top:{}%;width:{}%;height:{}%",hotspot.rect.x*100.,hotspot.rect.y*100.,hotspot.rect.width*100.,hotspot.rect.height*100.), "aria-label": "Interact with {hotspot.name}",
                                onclick: move |_| { if let Some(state) = session.write().as_mut() { if let Err(e) = state.activate(&hotspot.id) { state.message = e; } } }, "{hotspot.name}"
                            }
                        }
                    }
                }
                div { class: "inventory", strong { "INVENTORY" }
                    if state.inventory.is_empty() { span { class: "muted", "Nothing collected yet" } }
                    for item in state.project.items.iter().filter(|i| state.inventory.contains(&i.id)) { span { class: "chip", title: "{item.description}", "{item.name}" } }
                }
                if let Some(d) = dialogue {
                    div { class: "dialogue", strong { "{d.speaker}" } p { "{d.text}" }
                        for choice in d.choices.clone() {
                            button { onclick: move |_| { if let Some(state) = session.write().as_mut() { if let Err(e) = state.choose(&choice.id) { state.message = e; } } }, "{choice.text}" }
                        }
                        if d.choices.is_empty() { button { onclick: move |_| { if let Some(state) = session.write().as_mut() { if let Err(e) = state.continue_dialogue() { state.message = e; } } }, "Continue" } }
                    }
                }
                p { role: "status", "{state.message}" }
            }
        }
    }
}
