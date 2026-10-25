use app::{
    editor::{Editor, Selection},
    model::*,
};
fn background() -> Background {
    Background::import(
        "foyer.png".into(),
        include_bytes!("../../fixtures/backgrounds/foyer.png"),
    )
    .unwrap()
}
fn project() -> Project {
    let mut p = Project::default();
    let room = p.add_room();
    p.room_mut(&room).unwrap().background = Some(background());
    let item = p.add_item();
    let dialogue = p.add_dialogue();
    let choice = p.allocate_id("choice");
    p.dialogues[0].choices.push(Choice {
        id: choice,
        text: "Thanks!".into(),
        next: Some(dialogue.clone()),
        give_item: Some(item.clone()),
    });
    for action in [
        Action::Look {
            text: "A quiet room 🗝".into(),
        },
        Action::GoTo {
            room_id: room.clone(),
        },
        Action::Take {
            item_id: item.clone(),
        },
        Action::Talk {
            dialogue_id: dialogue,
        },
    ] {
        let id = p
            .add_hotspot(
                &room,
                Rect {
                    x: 0.1,
                    y: 0.2,
                    width: 0.25,
                    height: 0.3,
                },
            )
            .unwrap();
        let h = p.rooms[0].hotspots.iter_mut().find(|h| h.id == id).unwrap();
        h.action = action;
        h.requires_item = Some(item.clone());
    }
    p
}
#[test]
fn portable_roundtrip_preserves_every_field_and_asset() {
    let p = project();
    assert!(p.validate().is_empty());
    let text = p.to_json().unwrap();
    let loaded = Project::from_json(&text).unwrap();
    assert_eq!(loaded, p);
    assert_eq!(loaded.to_json().unwrap(), text);
}
#[test]
fn rejects_future_versions_and_unknown_fields_without_replacing_open_project() {
    let p = project();
    let mut editor = Editor::new(p.clone());
    for text in [
        p.to_json()
            .unwrap()
            .replace("\"version\": 1", "\"version\": 2"),
        p.to_json()
            .unwrap()
            .replace("\"title\":", "\"future_field\": true, \"title\":"),
        "not json".into(),
    ] {
        assert!(editor.open(&text).is_err());
        assert_eq!(editor.project, p);
    }
    let text = p
        .to_json()
        .unwrap()
        .replace("\"speaker\":", "\"future_nested\": 42, \"speaker\":");
    assert!(Project::from_json(&text).is_err());
}
#[test]
fn duplicate_ids_are_rejected_and_allocated_ids_never_collide_after_reopen() {
    let mut p = project();
    p.items[0].id = p.rooms[0].id.clone();
    assert!(Project::from_json(&p.to_json().unwrap()).is_err());
    let mut p = Project::from_json(&project().to_json().unwrap()).unwrap();
    let old = p.rooms[0].id.clone();
    assert_ne!(p.add_room(), old);
}
#[test]
fn missing_references_are_preserved_and_point_to_affected_objects() {
    let mut p = project();
    let removed = p.items.remove(0);
    let loaded = Project::from_json(&p.to_json().unwrap()).unwrap();
    let issues = loaded.validate();
    assert!(issues.iter().any(|i| i.object_id.as_deref()
        == Some(p.rooms[0].hotspots[2].id.as_str())
        && i.field == "action.item_id"));
    assert!(issues.iter().any(|i| i.object_id.as_deref()
        == Some(p.dialogues[0].choices[0].id.as_str())
        && i.field == "give_item"));
    assert_eq!(
        loaded.rooms[0].hotspots[0].requires_item.as_deref(),
        Some(removed.id.as_str())
    );
}
#[test]
fn edits_delete_undo_redo_and_failed_open_are_transactional() {
    let mut e = Editor::new(project());
    let initial = e.project.clone();
    e.selection = Selection::Room(e.project.rooms[0].id.clone());
    e.delete_selection();
    assert!(e.project.rooms.is_empty());
    assert!(e.project.validate().iter().any(|i| i.field == "start_room"));
    e.undo();
    assert_eq!(e.project, initial);
    e.redo();
    assert!(e.project.rooms.is_empty());
    e.undo();
    e.edit(|p| p.title = "A new story".into());
    assert!(!e.can_redo());
    assert!(e.dirty);
}
#[test]
fn import_rejects_unsupported_corrupt_or_oversize_images() {
    assert!(Background::import("a.svg".into(), b"<svg></svg>").is_err());
    assert!(Background::import("bad.png".into(), b"\x89PNG\r\n\x1a\n").is_err());
    assert!(Background::import("huge.png".into(), &vec![0; MAX_IMAGE_BYTES + 1]).is_err());
    let mut p = project();
    p.rooms[0].background.as_mut().unwrap().width += 1;
    assert!(p.validate().iter().any(|i| i.field == "background"));
}
#[test]
fn canvas_geometry_handles_reverse_drag_clamping_and_tiny_gestures() {
    assert_eq!(
        Rect::from_drag((0.8, 0.9), (0.2, 0.3)).unwrap(),
        Rect {
            x: 0.2,
            y: 0.3,
            width: 0.6000000000000001,
            height: 0.6000000000000001
        }
    );
    let rect = Rect::from_drag((-1., -1.), (2., 2.)).unwrap();
    assert_eq!(
        rect,
        Rect {
            x: 0.,
            y: 0.,
            width: 1.,
            height: 1.
        }
    );
    assert!(Rect::from_drag((0.5, 0.5), (0.5, 0.6)).is_none());
    assert!(
        Rect {
            x: f64::NAN,
            y: 0.,
            width: 0.1,
            height: 0.1
        }
        .valid()
            == false
    );
    let moved = Rect {
        x: 0.1,
        y: 0.2,
        width: 0.25,
        height: 0.3,
    }
    .translated(2., -2.);
    assert_eq!(moved.x, 0.75);
    assert_eq!(moved.y, 0.);
}
#[test]
fn room_and_hotspot_operations_preserve_links_across_rename_and_report_deletion() {
    let mut p = project();
    let destination = p.add_room();
    p.rooms[0].hotspots[1].action = Action::GoTo {
        room_id: destination.clone(),
    };
    p.room_mut(&destination).unwrap().name = "Renamed".into();
    assert!(!p.validate().iter().any(|i| i.field == "action.room_id"));
    let affected = p.rooms[0].hotspots[1].id.clone();
    let mut e = Editor::new(p.clone());
    e.selection = Selection::Room(destination);
    e.delete_selection();
    assert!(e
        .project
        .validate()
        .iter()
        .any(|i| i.object_id == Some(affected.clone()) && i.field == "action.room_id"));
    e.focus(Some(&affected));
    assert_eq!(
        e.selection,
        Selection::Hotspot(p.rooms[0].id.clone(), affected)
    );
    e.undo();
    assert_eq!(e.project, p);
}
