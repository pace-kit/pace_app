use app::{
    editor::Editor,
    model::Project,
    sharing::{server_url, RemoteSnapshot, SharingSession},
};

fn snapshot(title: &str, etag: &str) -> RemoteSnapshot {
    let mut project = Project::default();
    project.title = title.into();
    RemoteSnapshot {
        server: "https://projects.example/".into(),
        id: "12345678-1234-1234-1234-123456789abc".into(),
        etag: etag.into(),
        project,
    }
}

#[test]
fn download_requires_an_explicit_choice_and_preserves_local_edits() {
    let mut editor = Editor::default();
    let mut session = SharingSession::default();
    session.uploaded(editor.document_id, &editor, snapshot("Original", "\"1\""));
    editor.edit(|p| p.title = "Local edits".into());
    session.downloaded(
        editor.document_id,
        &editor,
        snapshot("Someone else's edits", "\"2\""),
    );
    assert_eq!(editor.project.title, "Local edits");
    assert_eq!(session.linked(&editor).unwrap().etag, "\"1\"");
    session.keep_local(&editor).unwrap();
    assert_eq!(editor.project.title, "Local edits");
    assert!(editor.dirty);
    assert_eq!(session.linked(&editor).unwrap().etag, "\"2\"");
    editor.undo();
    assert_eq!(editor.project.title, "Untitled adventure");
}

#[test]
fn opening_downloaded_snapshot_links_its_revision_without_changing_the_format() {
    let project =
        Project::from_json(include_str!("../../fixtures/three-rooms.pace-draft.json")).unwrap();
    let remote = RemoteSnapshot {
        project: project.clone(),
        ..snapshot("", "\"7\"")
    };
    let mut editor = Editor::default();
    let mut session = SharingSession::default();
    session.downloaded(editor.document_id, &editor, remote);
    session.open_downloaded(&mut editor).unwrap();
    assert_eq!(editor.project, project);
    assert_eq!(session.linked(&editor).unwrap().etag, "\"7\"");
    assert!(editor.dirty);
    assert!(!editor.can_undo());
}

#[test]
fn late_results_cannot_link_or_replace_a_different_document() {
    let original = Editor::default();
    let mut current = Editor::default();
    let mut session = SharingSession::default();
    assert!(!session.uploaded(
        original.document_id,
        &current,
        snapshot("Late upload", "\"1\"")
    ));
    assert!(!session.downloaded(
        original.document_id,
        &current,
        snapshot("Late download", "\"2\"")
    ));
    assert!(session.linked(&current).is_none());
    assert!(session.open_downloaded(&mut current).is_err());
    session.uploaded(current.document_id, &current, snapshot("Uploaded", "\"1\""));
    let replacement = Editor::new(current.project.clone());
    assert!(session.linked(&replacement).is_none());
}

#[test]
fn edits_during_upload_stay_dirty_and_unrelated_revisions_cannot_be_adopted() {
    let mut editor = Editor::default();
    let document_id = editor.document_id;
    let sent = snapshot("Sent", "\"2\"");
    editor.edit(|p| p.title = "Edited during upload".into());
    let mut session = SharingSession::default();
    assert!(session.uploaded(document_id, &editor, sent));
    assert_ne!(session.linked(&editor).unwrap().project, editor.project);
    assert!(editor.dirty);
    let other = RemoteSnapshot {
        server: "https://other.example/".into(),
        ..snapshot("Other", "\"10\"")
    };
    session.downloaded(document_id, &editor, other);
    assert!(session.keep_local(&editor).is_err());
    assert_eq!(session.linked(&editor).unwrap().etag, "\"2\"");
}

#[test]
fn server_urls_require_tls_except_loopback_and_never_accept_embedded_credentials() {
    assert_eq!(
        server_url("https://projects.example/pace").unwrap(),
        "https://projects.example/pace/"
    );
    assert!(server_url("http://127.0.0.1:3000").is_ok());
    assert!(server_url("http://[::1]:3000").is_ok());
    for url in [
        "http://projects.example",
        "file:///tmp/project",
        "https://user:secret@example.com",
        "https://example.com?token=x",
        "https://example.com#token",
    ] {
        assert!(server_url(url).is_err(), "{url}");
    }
}
