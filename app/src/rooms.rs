use crate::{
    browser,
    editor::{Editor, Selection},
    model::{Action, Background, Hotspot, Rect, Room},
};
use dioxus::prelude::*;

pub fn update_room(mut editor: Signal<Editor>, change: impl FnOnce(&mut Room)) {
    let id = editor.read().selected_room().map(str::to_owned);
    if let Some(id) = id {
        editor.write().edit(|p| {
            if let Some(r) = p.room_mut(&id) {
                change(r);
            }
        });
    }
}
pub fn update_hotspot(mut editor: Signal<Editor>, change: impl FnOnce(&mut Hotspot)) {
    let selection = editor.read().selection.clone();
    if let Selection::Hotspot(room_id, id) = selection {
        editor.write().edit(|p| {
            if let Some(h) = p
                .room_mut(&room_id)
                .and_then(|r| r.hotspots.iter_mut().find(|h| h.id == id))
            {
                change(h);
            }
        });
    }
}
fn position(rect: Rect) -> String {
    format!(
        "left:{}%;top:{}%;width:{}%;height:{}%",
        rect.x * 100.,
        rect.y * 100.,
        rect.width * 100.,
        rect.height * 100.
    )
}
#[derive(Clone)]
enum Drag {
    Draw {
        start: (f64, f64),
        end: (f64, f64),
    },
    Move {
        id: String,
        start: (f64, f64),
        end: (f64, f64),
        original: Rect,
    },
}
impl Drag {
    fn rect(&self) -> Option<Rect> {
        match self {
            Self::Draw { start, end } => Rect::from_drag(*start, *end),
            Self::Move {
                start,
                end,
                original,
                ..
            } => Some(original.translated(end.0 - start.0, end.1 - start.1)),
        }
    }
    fn set_end(&mut self, point: (f64, f64)) {
        match self {
            Self::Draw { end, .. } | Self::Move { end, .. } => *end = point,
        }
    }
}

#[component]
pub fn RoomWorkspace(mut editor: Signal<Editor>, mut notice: Signal<String>) -> Element {
    let mut drawing = use_signal(|| false);
    let mut drag: Signal<Option<Drag>> = use_signal(|| None);
    let room = editor
        .read()
        .selected_room()
        .and_then(|id| editor.read().project.room(id).cloned());
    let Some(room) = room else {
        return rsx! { div { class: "empty-state", span { class: "empty-icon", "◇" } h1 { "Every adventure starts somewhere." } p { "Add a room, import its background, and bring your world to life." } } };
    };
    let selected_id = match &editor.read().selection {
        Selection::Hotspot(_, id) => Some(id.clone()),
        _ => None,
    };
    let dimensions = room
        .background
        .as_ref()
        .map(|b| format!("{} × {}", b.width, b.height))
        .unwrap_or_default();
    rsx! {
      div { class: "room-workspace", onkeydown: move |event| { if event.key() == Key::Escape { drag.set(None); drawing.set(false); } },
        div { class: "room-heading", div { h1 { "{room.name}" } p { "Set the scene. Give every detail a story." } }
            label { class: "button", "Import background"
                input { r#type: "file", accept: "image/png,image/jpeg,image/webp", "aria-label": "Import background", onchange: move |event: Event<FormData>| async move {
                    let room_id = editor.read().selected_room().map(str::to_owned);
                    let document_id = editor.read().document_id;
                    if let (Some(files), Some(room_id)) = (event.files(), room_id) {
                        if let Some(name) = files.files().first() {
                            match files.read_file(name).await {
                                Some(_) if editor.read().document_id != document_id => notice.set("Image import cancelled because a different project was opened.".into()),
                                Some(bytes) => match Background::import(name.clone(), &bytes) {
                                    Ok(background) => { editor.write().edit(|p| { if let Some(room) = p.room_mut(&room_id) { room.background = Some(background); } }); notice.set("Background imported. Hotspot bounds scale with the image.".into()); },
                                    Err(e) => notice.set(e),
                                },
                                None => notice.set("Cannot read this image.".into()),
                            }
                        }
                    }
                } }
            }
        }
        if let Some(background) = &room.background {
            div { class: "segmented",
                button { class: if !drawing() { "primary" } else { "" }, onclick: move |_| { drawing.set(false); drag.set(None); }, "Select / move" }
                button { class: if drawing() { "primary" } else { "" }, onclick: move |_| { drawing.set(true); drag.set(None); }, "Draw hotspot" }
                button { onclick: move |_| {
                    let room_id = editor.read().selected_room().map(str::to_owned);
                    if let Some(room_id) = room_id { let mut id = None; editor.write().edit(|p| id = p.add_hotspot(&room_id, Rect { x:0.35,y:0.35,width:0.2,height:0.2 })); if let Some(id) = id { editor.write().selection = Selection::Hotspot(room_id,id); } }
                }, "Add hotspot" }
            }
            div { id: "room-stage", tabindex: "0", class: if drawing() { "stage drawing" } else { "stage" }, style: "aspect-ratio:{background.width}/{background.height}", "aria-label": "Room canvas",
                onkeydown: move |event| { if event.key() == Key::Escape { drag.set(None); drawing.set(false); } },
                onpointerdown: move |event| { if drawing() && event.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) {
                    let point = event.client_coordinates(); if let Some(point) = browser::stage_point(point.x,point.y) { browser::capture_pointer(event.pointer_id()); drag.set(Some(Drag::Draw {start:point,end:point})); }
                } },
                onpointermove: move |event| { let point = event.client_coordinates(); if let Some(point) = browser::stage_point(point.x,point.y) { if let Some(drag) = drag.write().as_mut() { drag.set_end(point); } } },
                onpointercancel: move |_| drag.set(None),
                onpointerup: move |_| {
                    let gesture = drag.write().take();
                    if let Some(gesture) = gesture { if let Some(rect) = gesture.rect() {
                        let room_id = editor.read().selected_room().map(str::to_owned);
                        if let Some(room_id) = room_id { match gesture {
                            Drag::Draw {..} => { let mut id = None; editor.write().edit(|p| id = p.add_hotspot(&room_id,rect)); if let Some(id) = id { editor.write().selection = Selection::Hotspot(room_id,id); } drawing.set(false); },
                            Drag::Move {id,..} => editor.write().edit(|p| { if let Some(h) = p.room_mut(&room_id).and_then(|r| r.hotspots.iter_mut().find(|h| h.id == id)) { h.rect = rect; } }),
                        } }
                    } }
                },
                img { src: background.display_url(), alt: "{room.name} background", draggable: "false" }
                for hotspot in room.hotspots.clone() {
                    HotspotButton { key: "{hotspot.id}", hotspot: hotspot.clone(), selected: selected_id.as_deref() == Some(&hotspot.id), editor, drag }
                }
                if let Some(rect) = drag.read().as_ref().and_then(Drag::rect) { div { class: "ghost", style: position(rect) } }
            }
            div { class: "stage-caption", span { "{dimensions} · Original image coordinates" } span { "{room.hotspots.len()} hotspots" } }
            p { class: "muted", style: "margin-top:14px", "Draw a rectangle, drag to move, or use Add hotspot and edit its bounds. Escape cancels a gesture." }
        } else {
            div { class: "empty-state", span { class: "empty-icon", "▧" } h1 { "A place for your story." } p { "Import a PNG, JPEG, or WebP background to start placing hotspots." } p { class: "muted", "Up to 8 MiB per image. Images travel with your project." } }
        }
        if !room.hotspots.is_empty() {
            div { class: "hotspot-list", h3 { "IN THIS ROOM" }
                for hotspot in room.hotspots {
                    button { class: "hotspot-row", onclick: move |_| {
                        let room_id = editor.read().selected_room().map(str::to_owned); if let Some(room_id) = room_id { editor.write().selection = Selection::Hotspot(room_id,hotspot.id.clone()); }
                    }, "◇  {hotspot.name}" span { {match hotspot.action { Action::Look {..} => "INSPECT", Action::GoTo {..} => "GO TO ROOM", Action::Take {..} => "PICK UP ITEM", Action::Talk {..} => "DIALOGUE" }} } }
                }
            }
        }
      }
    }
}

#[component]
fn HotspotButton(
    hotspot: Hotspot,
    selected: bool,
    mut editor: Signal<Editor>,
    mut drag: Signal<Option<Drag>>,
) -> Element {
    let id = hotspot.id.clone();
    let key_id = id.clone();
    let click_id = id.clone();
    rsx! { button { class: if selected { "hotspot selected" } else { "hotspot" }, style: position(hotspot.rect), "aria-label": "Select hotspot {hotspot.name}",
        onclick: move |_| { let room_id = editor.read().selected_room().map(str::to_owned); if let Some(room_id) = room_id { editor.write().selection = Selection::Hotspot(room_id,click_id.clone()); } },
        onpointerdown: move |event| {
            event.stop_propagation();
            let room_id = editor.read().selected_room().map(str::to_owned);
            if let Some(room_id) = room_id { editor.write().selection = Selection::Hotspot(room_id,id.clone()); }
            let point = event.client_coordinates(); if let Some(point) = browser::stage_point(point.x,point.y) { browser::capture_pointer(event.pointer_id()); drag.set(Some(Drag::Move { id:id.clone(),start:point,end:point,original:hotspot.rect })); }
        },
        onkeydown: move |event| {
            let delta = match event.key() { Key::ArrowLeft => Some((-0.01,0.)), Key::ArrowRight => Some((0.01,0.)), Key::ArrowUp => Some((0.,-0.01)), Key::ArrowDown => Some((0.,0.01)), Key::Escape => { drag.set(None); None }, _ => None };
            if let Some((dx,dy)) = delta { let room_id = editor.read().selected_room().map(str::to_owned); if let Some(room_id) = room_id { editor.write().edit(|p| { if let Some(h) = p.room_mut(&room_id).and_then(|r| r.hotspots.iter_mut().find(|h| h.id == key_id)) { h.rect = h.rect.translated(dx,dy); } }); } }
        },
        "{hotspot.name}"
    } }
}

#[component]
pub fn RoomInspector(mut editor: Signal<Editor>) -> Element {
    let room = editor
        .read()
        .selected_room()
        .and_then(|id| editor.read().project.room(id).cloned());
    let room = room?;
    let hotspot = match &editor.read().selection {
        Selection::Hotspot(_, id) => room.hotspots.iter().find(|h| &h.id == id).cloned(),
        _ => None,
    };
    if let Some(h) = hotspot {
        let action = match h.action {
            Action::Look { .. } => "look",
            Action::GoTo { .. } => "go_to",
            Action::Take { .. } => "take",
            Action::Talk { .. } => "talk",
        };
        rsx! {
            h2 { "Hotspot" } p { class: "muted", "{h.id}" }
            label { "Hotspot name" input { value: "{h.name}", oninput: move |e| update_hotspot(editor,|h| h.name = e.value()) } }
            h3 { "BOUNDS · % OF IMAGE" }
            div { class: "fields-grid",
                for (field,value) in [("X",h.rect.x),("Y",h.rect.y),("Width",h.rect.width),("Height",h.rect.height)] {
                    label { "{field} (%)" input { r#type: "number", min: "0", max: "100", step: "0.1", value: format!("{:.2}",value*100.), oninput: move |e| { if let Ok(value) = e.value().parse::<f64>() { update_hotspot(editor, |h| match field { "X" => h.rect.x = value/100., "Y" => h.rect.y = value/100., "Width" => h.rect.width = value/100., _ => h.rect.height = value/100. }); } } } }
                }
            }
            label { "Action" select { value: action, onchange: move |e| update_hotspot(editor, |h| h.action = match e.value().as_str() { "go_to" => Action::GoTo { room_id: String::new() }, "take" => Action::Take { item_id: String::new() }, "talk" => Action::Talk { dialogue_id: String::new() }, _ => Action::Look { text: String::new() } }),
                option { value: "look", selected: action == "look", "Inspect" } option { value: "go_to", selected: action == "go_to", "Go to room" } option { value: "take", selected: action == "take", "Pick up item" } option { value: "talk", selected: action == "talk", "Start dialogue" }
            } }
            match h.action {
                Action::Look {text} => rsx! { label { "Inspection text" textarea { value: "{text}", oninput: move |e| update_hotspot(editor, |h| h.action = Action::Look { text:e.value() }) } } },
                Action::GoTo {room_id} => rsx! { label { "Destination room" select { value: "{room_id}", onchange: move |e| update_hotspot(editor, |h| h.action = Action::GoTo { room_id:e.value() }), option { value: "", selected: room_id.is_empty(), "Choose destination" } for r in editor.read().project.rooms.clone() { option { value: "{r.id}", selected: r.id == room_id, "{r.name}" } } } } },
                Action::Take {item_id} => rsx! { label { "Item to pick up" select { value: "{item_id}", onchange: move |e| update_hotspot(editor, |h| h.action = Action::Take { item_id:e.value() }), option { value: "", selected: item_id.is_empty(), "Choose item" } for item in editor.read().project.items.clone() { option { value: "{item.id}", selected: item.id == item_id, "{item.name}" } } } } },
                Action::Talk {dialogue_id} => rsx! { label { "Dialogue to start" select { value: "{dialogue_id}", onchange: move |e| update_hotspot(editor, |h| h.action = Action::Talk { dialogue_id:e.value() }), option { value: "", selected: dialogue_id.is_empty(), "Choose dialogue" } for d in editor.read().project.dialogues.clone() { option { value: "{d.id}", selected: d.id == dialogue_id, "{d.name}" } } } } },
            }
            label { "Requires item" select { value: h.requires_item.clone().unwrap_or_default(), onchange: move |e| update_hotspot(editor, |h| h.requires_item = if e.value().is_empty() { None } else { Some(e.value()) }), option { value: "", selected: h.requires_item.is_none(), "No requirement" } for item in editor.read().project.items.clone() { option { value: "{item.id}", selected: h.requires_item.as_deref() == Some(item.id.as_str()), "{item.name}" } } } }
            p { class: "muted", "Room links are one-way. Add a return hotspot in the destination room." }
            button { class: "danger", onclick: move |_| editor.write().delete_selection(), "Delete hotspot" }
        }
    } else {
        rsx! { h2 { "Room" } p { class: "muted", "{room.id}" }
            label { "Room name" input { value: "{room.name}", oninput: move |e| update_room(editor, |r| r.name = e.value()) } }
            p { class: "muted", "Select a hotspot on the canvas or in the list to edit its action and bounds." }
            button { class: "danger", onclick: move |_| { if browser::confirm("Delete this room and its hotspots? Links from other objects will be reported in validation. You can undo this.") { editor.write().delete_selection(); } }, "Delete room" }
        }
    }
}
