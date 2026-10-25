use crate::{
    browser,
    editor::{Editor, Selection},
    model::{Choice, Dialogue, Item},
};
use dioxus::prelude::*;

fn update_item(mut editor: Signal<Editor>, change: impl FnOnce(&mut Item)) {
    let selection = editor.read().selection.clone();
    if let Selection::Item(id) = selection {
        editor.write().edit(|p| {
            if let Some(item) = p.items.iter_mut().find(|i| i.id == id) {
                change(item);
            }
        });
    }
}
fn update_dialogue(mut editor: Signal<Editor>, change: impl FnOnce(&mut Dialogue)) {
    let selection = editor.read().selection.clone();
    if let Selection::Dialogue(id) = selection {
        editor.write().edit(|p| {
            if let Some(dialogue) = p.dialogues.iter_mut().find(|d| d.id == id) {
                change(dialogue);
            }
        });
    }
}

#[component]
pub fn StoryWorkspace(mut editor: Signal<Editor>) -> Element {
    let selection = editor.read().selection.clone();
    match selection {
        Selection::Item(id) => {
            let item = editor
                .read()
                .project
                .items
                .iter()
                .find(|i| i.id == id)
                .cloned();
            let item = item?;
            rsx! { div { class: "room-heading", div { h1 { "{item.name}" } p { "Something to find. Something to take along." } } }
                div { class: "card", h2 { "Inventory item" } p { class: "muted", "{item.id}" }
                    label { "Item name" input { value: "{item.name}", oninput: move |e| update_item(editor, |i| i.name = e.value()) } }
                    label { "Item description" textarea { value: "{item.description}", oninput: move |e| update_item(editor, |i| i.description = e.value()) } }
                    p { "Use a Pick up item hotspot or a dialogue reward to add this item to the player's inventory. A hotspot can require it before its action is available." }
                    button { class: "danger", onclick: move |_| { if browser::confirm("Delete this item? References will appear in validation. You can undo this.") { editor.write().delete_selection(); } }, "Delete item" }
                }
            }
        }
        Selection::Dialogue(id) => {
            let dialogue = editor
                .read()
                .project
                .dialogues
                .iter()
                .find(|d| d.id == id)
                .cloned();
            let d = dialogue?;
            rsx! { div { class: "room-heading", div { h1 { "{d.name}" } p { "A conversation can open a whole new world." } } }
                div { class: "card", h2 { "Dialogue node" } p { class: "muted", "{d.id}" }
                    label { "Dialogue name" input { value: "{d.name}", oninput: move |e| update_dialogue(editor, |d| d.name = e.value()) } }
                    label { "Speaker" input { value: "{d.speaker}", oninput: move |e| update_dialogue(editor, |d| d.speaker = e.value()) } }
                    label { "Dialogue text" textarea { value: "{d.text}", oninput: move |e| update_dialogue(editor, |d| d.text = e.value()) } }
                    h3 { "PLAYER CHOICES" }
                    p { class: "muted", "Each choice can lead to another node and grant an item. No choices gives the player a Continue button to end the conversation." }
                    for choice in d.choices { ChoiceEditor { key: "{choice.id}", choice, editor } }
                    button { onclick: move |_| {
                        let selection = editor.read().selection.clone();
                        if let Selection::Dialogue(dialogue_id) = selection { editor.write().edit(|p| {
                            let id = p.allocate_id("choice");
                            if let Some(d) = p.dialogues.iter_mut().find(|d| d.id == dialogue_id) { d.choices.push(Choice { id, text: "New choice".into(), next: None, give_item: None }); }
                        }); }
                    }, "Add choice" }
                    br {}
                    button { class: "danger", onclick: move |_| { if browser::confirm("Delete this dialogue? References will appear in validation. You can undo this.") { editor.write().delete_selection(); } }, "Delete dialogue" }
                }
            }
        }
        _ => rsx! {},
    }
}

#[component]
fn ChoiceEditor(choice: Choice, editor: Signal<Editor>) -> Element {
    let id = choice.id.clone();
    let next_id = id.clone();
    let item_id = id.clone();
    let delete_id = id.clone();
    rsx! { div { class: "choice", "data-choice-id": "{choice.id}",
        label { "Choice text" input { value: "{choice.text}", oninput: move |e| update_dialogue(editor, |d| { if let Some(c) = d.choices.iter_mut().find(|c| c.id == id) { c.text = e.value(); } }) } }
        label { "Next dialogue" select { value: choice.next.clone().unwrap_or_default(), onchange: move |e| update_dialogue(editor, |d| { if let Some(c) = d.choices.iter_mut().find(|c| c.id == next_id) { c.next = if e.value().is_empty() { None } else { Some(e.value()) }; } }),
            option { value: "", selected: choice.next.is_none(), "End conversation" }
            for dialogue in editor.read().project.dialogues.clone() { option { value: "{dialogue.id}", selected: choice.next.as_deref() == Some(dialogue.id.as_str()), "{dialogue.name}" } }
        } }
        label { "Give item" select { value: choice.give_item.clone().unwrap_or_default(), onchange: move |e| update_dialogue(editor, |d| { if let Some(c) = d.choices.iter_mut().find(|c| c.id == item_id) { c.give_item = if e.value().is_empty() { None } else { Some(e.value()) }; } }),
            option { value: "", selected: choice.give_item.is_none(), "No item" }
            for item in editor.read().project.items.clone() { option { value: "{item.id}", selected: choice.give_item.as_deref() == Some(item.id.as_str()), "{item.name}" } }
        } }
        button { onclick: move |_| update_dialogue(editor, |d| d.choices.retain(|c| c.id != delete_id)), "Remove choice" }
    } }
}
