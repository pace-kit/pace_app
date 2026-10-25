use app::{model::*, rehearsal::Rehearsal};
fn sample() -> Project {
    let mut p = Project::default();
    let a = p.add_room();
    let b = p.add_room();
    for r in &mut p.rooms {
        r.background = Some(
            Background::import(
                "test.png".into(),
                include_bytes!("../../fixtures/backgrounds/foyer.png"),
            )
            .unwrap(),
        );
    }
    let item = p.add_item();
    p.items[0].name = "Brass key".into();
    let dialogue = p.add_dialogue();
    let choice = p.allocate_id("choice");
    p.dialogues[0].choices.push(Choice {
        id: choice,
        text: "Take the key".into(),
        next: None,
        give_item: Some(item.clone()),
    });
    let rect = Rect {
        x: 0.1,
        y: 0.1,
        width: 0.2,
        height: 0.2,
    };
    p.add_hotspot(&a, rect);
    p.add_hotspot(&a, rect);
    p.add_hotspot(&a, rect);
    p.rooms[0].hotspots[0].action = Action::GoTo { room_id: b };
    p.rooms[0].hotspots[0].requires_item = Some(item.clone());
    p.rooms[0].hotspots[1].action = Action::Talk {
        dialogue_id: dialogue,
    };
    p.rooms[0].hotspots[2].action = Action::Take { item_id: item };
    p
}
#[test]
fn item_gate_dialogue_reward_and_room_transition_do_not_mutate_project() {
    let p = sample();
    let original = p.clone();
    let mut s = Rehearsal::new(&p).unwrap();
    let door = &p.rooms[0].hotspots[0].id;
    s.activate(door).unwrap();
    assert_eq!(s.room_id, p.rooms[0].id);
    assert_eq!(s.message, "You need Brass key first.");
    assert!(s.choose("not active").is_err());
    s.activate(&p.rooms[0].hotspots[1].id).unwrap();
    assert!(s.activate(door).is_err());
    assert!(s.continue_dialogue().is_err());
    s.choose(&p.dialogues[0].choices[0].id).unwrap();
    assert!(s.inventory.contains(&p.items[0].id));
    assert_eq!(s.dialogue_id, None);
    s.activate(door).unwrap();
    assert_eq!(s.room_id, p.rooms[1].id);
    assert!(s.activate(door).is_err());
    assert_eq!(p, original);
    assert_eq!(s.project, original);
    let reset = Rehearsal::new(&p).unwrap();
    assert!(reset.inventory.is_empty());
    assert_eq!(reset.room_id, p.rooms[0].id);
}
#[test]
fn pickup_is_idempotent_and_invalid_projects_cannot_start() {
    let mut p = sample();
    let mut s = Rehearsal::new(&p).unwrap();
    let pickup = &p.rooms[0].hotspots[2].id;
    s.activate(pickup).unwrap();
    s.activate(pickup).unwrap();
    assert_eq!(s.inventory.len(), 1);
    assert!(s.message.starts_with("You already have"));
    p.items.clear();
    assert!(Rehearsal::new(&p).is_err());
}
#[test]
fn dialogue_branches_and_terminal_continue_are_explicit() {
    let mut p = sample();
    let terminal = p.add_dialogue();
    p.dialogues[0].choices[0].next = Some(terminal.clone());
    let mut s = Rehearsal::new(&p).unwrap();
    s.activate(&p.rooms[0].hotspots[1].id).unwrap();
    s.choose(&p.dialogues[0].choices[0].id).unwrap();
    assert_eq!(s.dialogue_id, Some(terminal));
    s.continue_dialogue().unwrap();
    assert!(s.dialogue_id.is_none());
}
