use super::{server_url, RemoteClient, SharingSession};
use crate::{browser, editor::Editor};
use dioxus::prelude::*;

#[component]
pub fn SharingPanel(mut editor: Signal<Editor>) -> Element {
    let mut server = use_signal(|| "http://127.0.0.1:3000".to_owned());
    let mut token = use_signal(String::new);
    let mut project_id = use_signal(String::new);
    let mut member = use_signal(String::new);
    let mut role = use_signal(|| "reader".to_owned());
    let mut session = use_signal(SharingSession::default);
    let mut busy = use_signal(|| false);
    let mut status = use_signal(String::new);
    let linked = session
        .read()
        .linked(&editor.read())
        .cloned()
        .filter(|remote| {
            server_url(&server.read()).as_ref() == Ok(&remote.server)
                && *project_id.read() == remote.id
        });
    let staged = session
        .read()
        .staged(&editor.read())
        .cloned()
        .filter(|remote| {
            server_url(&server.read()).as_ref() == Ok(&remote.server)
                && *project_id.read() == remote.id
        });
    let can_upload = linked.is_some();
    let can_keep = linked.is_some() && session.read().can_keep_local(&editor.read());
    rsx! {
        details { class: "sharing-panel",
            summary { "Project sharing" }
            p { "Share complete project snapshots. Changes are sent only when you upload; other clients download them explicitly." }
            div { class: "sharing-fields",
                label { "Server URL" input { value: "{server}", disabled: busy(), oninput: move |e| server.set(e.value()) } }
                label { "Bearer token" input { r#type: "password", autocomplete: "off", value: "{token}", disabled: busy(), oninput: move |e| token.set(e.value()) } }
                label { "Shared project ID" input { value: "{project_id}", disabled: busy(), oninput: move |e| project_id.set(e.value()) } }
            }
            div { class: "inline-actions",
                button { disabled: busy(), onclick: move |_| async move {
                    let client = match RemoteClient::new(&server.read(), &token.read()) { Ok(c) => c, Err(e) => { status.set(e.to_string()); return; } };
                    let document_id = editor.read().document_id;
                    let project = editor.read().project.clone();
                    busy.set(true);
                    status.set("Uploading a new project…".into());
                    let result = client.create(project).await;
                    busy.set(false);
                    match result {
                        Ok(snapshot) => {
                            let id = snapshot.id.clone();
                            if session.write().uploaded(document_id, &editor.read(), snapshot) {
                                project_id.set(id);
                                status.set("Project uploaded. Grant access to a member, then share the project ID.".into());
                            } else { status.set(format!("Uploaded project {id}. Your open document changed during the request and was not linked.")); }
                        }
                        Err(e) => status.set(e.to_string()),
                    }
                }, "Upload as new project" }
                button { disabled: busy() || !can_upload, onclick: move |_| async move {
                    let client = match RemoteClient::new(&server.read(), &token.read()) { Ok(c) => c, Err(e) => { status.set(e.to_string()); return; } };
                    let Some(previous) = session.read().linked(&editor.read()).cloned() else { return; };
                    let document_id = editor.read().document_id;
                    let project = editor.read().project.clone();
                    busy.set(true);
                    status.set("Uploading changes…".into());
                    let result = client.upload(&previous, project).await;
                    busy.set(false);
                    match result {
                        Ok(snapshot) => {
                            if session.write().uploaded(document_id, &editor.read(), snapshot) {
                                status.set("Changes uploaded. Edits made while uploading remain open; save a local file to keep a copy.".into());
                            } else { status.set("Upload completed for the previous document; your open document is unchanged.".into()); }
                        }
                        Err(e) => status.set(e.to_string()),
                    }
                }, "Upload changes" }
                button { disabled: busy(), onclick: move |_| async move {
                    let client = match RemoteClient::new(&server.read(), &token.read()) { Ok(c) => c, Err(e) => { status.set(e.to_string()); return; } };
                    let id = project_id.read().clone();
                    let document_id = editor.read().document_id;
                    session.write().staged = None;
                    busy.set(true);
                    status.set("Downloading project…".into());
                    let result = client.download(&id).await;
                    busy.set(false);
                    match result {
                        Ok(snapshot) => {
                            if session.write().downloaded(document_id, &editor.read(), snapshot) {
                                status.set("Download ready. Your open project is unchanged. Choose whether to open it or keep your local edits.".into());
                            } else { status.set("Download discarded because a different document was opened. Download again to continue.".into()); }
                        }
                        Err(e) => status.set(e.to_string()),
                    }
                }, "Download project" }
                button { disabled: busy(), onclick: move |_| { token.set(String::new()); status.set("Token cleared from this editor session.".into()); }, "Clear token" }
            }
            if let Some(remote) = linked {
                p { class: "muted", "Linked revision {remote.etag}. "
                    if remote.project == editor.read().project { "The open project matches this snapshot." }
                    else { "You have local edits to upload." }
                }
            }
            if let Some(remote) = staged {
                div { class: "sharing-download",
                    p { "Downloaded “{remote.project.title}” · revision {remote.etag}" }
                    div { class: "inline-actions",
                        button { disabled: busy(), onclick: move |_| {
                            if editor.read().dirty && !browser::confirm("Replace your open project with the downloaded snapshot? Save a local file first to keep your edits.") { return; }
                            match session.write().open_downloaded(&mut editor.write()) {
                                Ok(()) => status.set("Downloaded project opened. Save a project file to keep a local copy.".into()),
                                Err(e) => status.set(e.into()),
                            }
                        }, "Open downloaded project" }
                        button { disabled: busy() || !can_keep, onclick: move |_| {
                            if !browser::confirm("Keep your local edits against this downloaded revision? Your next upload will replace that remote snapshot.") { return; }
                            match session.write().keep_local(&editor.read()) {
                                Ok(()) => status.set("Local edits kept against the downloaded revision. Review them, then choose Upload changes.".into()),
                                Err(e) => status.set(e.into()),
                            }
                        }, "Keep local edits against downloaded revision" }
                    }
                }
            }
            div { class: "sharing-fields",
                label { "Member ID" input { value: "{member}", disabled: busy(), oninput: move |e| member.set(e.value()) } }
                label { "Member access" select { "aria-label": "Member access", value: "{role}", disabled: busy(), onchange: move |e| role.set(e.value()), option { value: "reader", "Reader" } option { value: "writer", "Writer" } } }
                div { class: "inline-actions",
                    button { disabled: busy(), onclick: move |_| async move {
                        let client = match RemoteClient::new(&server.read(), &token.read()) { Ok(c) => c, Err(e) => { status.set(e.to_string()); return; } };
                        let (id, member, role) = (project_id.read().clone(), member.read().clone(), role.read().clone());
                        busy.set(true);
                        let result = client.set_access(&id, &member, Some(&role)).await;
                        busy.set(false);
                        status.set(match result { Ok(()) => format!("Granted {role} access to {member}."), Err(e) => e.to_string() });
                    }, "Grant access" }
                    button { disabled: busy(), onclick: move |_| async move {
                        let client = match RemoteClient::new(&server.read(), &token.read()) { Ok(c) => c, Err(e) => { status.set(e.to_string()); return; } };
                        let (id, member) = (project_id.read().clone(), member.read().clone());
                        busy.set(true);
                        let result = client.set_access(&id, &member, None).await;
                        busy.set(false);
                        status.set(match result { Ok(()) => format!("Revoked access for {member}."), Err(e) => e.to_string() });
                    }, "Revoke access" }
                }
            }
            p { class: "muted", "Only the owner can change access. Tokens are held in memory and cleared when you reload. Sharing does not save your local file." }
            p { role: "status", "data-testid": "sharing-status", "{status}" }
        }
    }
}
