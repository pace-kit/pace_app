//! Snapshot sharing of the existing editor format; no live edits or automatic retries.
mod ui;
pub use ui::SharingPanel;

use crate::{editor::Editor, model::Project};
use serde::Deserialize;

#[derive(Clone, Debug, PartialEq)]
pub struct RemoteSnapshot {
    pub server: String,
    pub id: String,
    pub etag: String,
    pub project: Project,
}

#[derive(Clone, Default)]
pub struct SharingSession {
    linked: Option<(usize, RemoteSnapshot)>,
    staged: Option<(usize, RemoteSnapshot)>,
}

impl SharingSession {
    pub fn linked(&self, editor: &Editor) -> Option<&RemoteSnapshot> {
        self.linked
            .as_ref()
            .filter(|(id, _)| *id == editor.document_id)
            .map(|(_, v)| v)
    }
    pub fn staged(&self, editor: &Editor) -> Option<&RemoteSnapshot> {
        self.staged
            .as_ref()
            .filter(|(id, _)| *id == editor.document_id)
            .map(|(_, v)| v)
    }
    pub fn uploaded(
        &mut self,
        document_id: usize,
        editor: &Editor,
        snapshot: RemoteSnapshot,
    ) -> bool {
        if document_id != editor.document_id {
            return false;
        }
        self.linked = Some((document_id, snapshot));
        self.staged = None;
        true
    }
    pub fn downloaded(
        &mut self,
        document_id: usize,
        editor: &Editor,
        snapshot: RemoteSnapshot,
    ) -> bool {
        if document_id != editor.document_id {
            return false;
        }
        self.staged = Some((document_id, snapshot));
        true
    }
    /// Downloading never changes the open document or its upload precondition.
    pub fn open_downloaded(&mut self, editor: &mut Editor) -> Result<(), &'static str> {
        let snapshot = self
            .staged(editor)
            .cloned()
            .ok_or("Download the project first")?;
        *editor = Editor::new(snapshot.project.clone());
        editor.dirty = true; // Still needs a local file/recovery save.
        self.linked = Some((editor.document_id, snapshot));
        self.staged = None;
        Ok(())
    }
    pub fn can_keep_local(&self, editor: &Editor) -> bool {
        matches!((self.linked(editor), self.staged(editor)), (Some(a), Some(b)) if a.server == b.server && a.id == b.id)
    }
    /// Only an explicit reconciliation decision advances the revision for local edits.
    pub fn keep_local(&mut self, editor: &Editor) -> Result<(), &'static str> {
        if !self.can_keep_local(editor) {
            return Err("Download the linked project's latest revision first");
        }
        self.linked = self.staged.take();
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShareError {
    Conflict(i64),
    Message(String),
}

impl std::fmt::Display for ShareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict(revision) => write!(f, "Conflict: the server has revision {revision}. Your edits are safe. Download the latest project, then choose which version to keep."),
            Self::Message(message) => f.write_str(message),
        }
    }
}

fn message(text: &str) -> ShareError {
    ShareError::Message(text.into())
}

pub fn server_url(value: &str) -> Result<String, ShareError> {
    let mut url =
        url::Url::parse(value.trim()).map_err(|_| message("Enter an absolute server URL"))?;
    let loopback = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if (url.scheme() != "https" && !(url.scheme() == "http" && loopback))
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(message(
            "Use HTTPS (HTTP is allowed on localhost), without credentials, query or fragment",
        ));
    }
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url.to_string())
}

fn valid_id(id: &str) -> bool {
    id.len() == 36
        && id.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_hexdigit()
            }
        })
}

fn revision(etag: Option<String>) -> Result<String, ShareError> {
    let etag = etag.ok_or_else(|| {
        message("Server did not expose its revision ETag; check the server's allowed origins")
    })?;
    let number = etag
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .and_then(|s| s.parse::<i64>().ok());
    if !number.is_some_and(|n| n > 0 && etag == format!("\"{n}\"")) {
        return Err(message("Server returned an invalid revision ETag"));
    }
    Ok(etag)
}

#[derive(Deserialize)]
struct HttpResponse {
    status: u16,
    etag: Option<String>,
    text: String,
}

impl HttpResponse {
    fn expect(self, expected: u16) -> Result<Self, ShareError> {
        if self.status == expected {
            return Ok(self);
        }
        if self.status == 412 {
            let error: serde_json::Value = serde_json::from_str(&self.text).unwrap_or_default();
            if let Some(revision) = error["current_revision"].as_i64().filter(|n| *n > 0) {
                return Err(ShareError::Conflict(revision));
            }
        }
        Err(message(match self.status {
            401 => "Authentication failed. Check your bearer token.",
            403 => "You have read-only access to this project.",
            404 => "Project not found or access has not been granted.",
            413 => "Project exceeds the server's upload limit (64 MiB on the current server).",
            400 | 415 => "The server rejected the project or request. Check its format and server version.",
            412 | 428 => "The project revision is missing or outdated. Download the latest project before uploading.",
            _ => "The server could not complete the request. Keep your edits and check the remote project before retrying.",
        }))
    }
}

pub struct RemoteClient {
    server: String,
    token: String,
}

impl RemoteClient {
    pub fn new(server: &str, token: &str) -> Result<Self, ShareError> {
        if token.is_empty()
            || token.len() > 512
            || !token
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._~+/=".contains(&b))
        {
            return Err(message("Enter a valid bearer token"));
        }
        Ok(Self {
            server: server_url(server)?,
            token: token.to_owned(),
        })
    }
    fn project_path(&self, id: &str) -> Result<String, ShareError> {
        if !valid_id(id) {
            return Err(message("Enter the shared project's UUID"));
        }
        Ok(format!("v1/projects/{id}"))
    }
    async fn request(
        &self,
        method: &str,
        path: &str,
        etag: Option<&str>,
        body: Option<&str>,
    ) -> Result<HttpResponse, ShareError> {
        let url = format!("{}{path}", self.server);
        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::prelude::*;
            #[wasm_bindgen(module = "/src/sharing/http.js")]
            extern "C" {
                #[wasm_bindgen(catch, js_name = projectRequest)]
                async fn request(
                    method: &str,
                    url: &str,
                    token: &str,
                    etag: Option<String>,
                    body: Option<String>,
                    limit: usize,
                ) -> Result<JsValue, JsValue>;
            }
            let value = request(method, &url, &self.token, etag.map(str::to_owned), body.map(str::to_owned), crate::model::MAX_PROJECT_BYTES).await
                .map_err(|_| message("Request failed or timed out. Check the connection and allowed origins. An upload may have reached the server; download before retrying."))?;
            serde_wasm_bindgen::from_value(value).map_err(|_| message("Invalid server response"))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = (method, url, &self.token, etag, body);
            Err(message("Project sharing requires the web platform"))
        }
    }
    pub async fn create(&self, project: Project) -> Result<RemoteSnapshot, ShareError> {
        let body = project.to_json().map_err(ShareError::Message)?;
        let response = self
            .request("POST", "v1/projects", None, Some(&body))
            .await?
            .expect(201)?;
        #[derive(Deserialize)]
        struct Created {
            id: String,
            revision: i64,
        }
        let created: Created =
            serde_json::from_str(&response.text).map_err(|_| message("Invalid create response"))?;
        let etag = revision(response.etag)?;
        if !valid_id(&created.id) || etag != format!("\"{}\"", created.revision) {
            return Err(message("Invalid create revision"));
        }
        Ok(RemoteSnapshot {
            server: self.server.clone(),
            id: created.id,
            etag,
            project,
        })
    }
    pub async fn upload(
        &self,
        previous: &RemoteSnapshot,
        project: Project,
    ) -> Result<RemoteSnapshot, ShareError> {
        if self.server != previous.server {
            return Err(message("Download the project from this server first"));
        }
        let body = project.to_json().map_err(ShareError::Message)?;
        let response = self
            .request(
                "PUT",
                &self.project_path(&previous.id)?,
                Some(&previous.etag),
                Some(&body),
            )
            .await?
            .expect(204)?;
        Ok(RemoteSnapshot {
            etag: revision(response.etag)?,
            project,
            ..previous.clone()
        })
    }
    pub async fn download(&self, id: &str) -> Result<RemoteSnapshot, ShareError> {
        let response = self
            .request("GET", &self.project_path(id)?, None, None)
            .await?
            .expect(200)?;
        let etag = revision(response.etag)?;
        let project = Project::from_json(&response.text).map_err(|_| message("Downloaded data is not a supported editor draft. Your open project is unchanged; core scenes use the core loader."))?;
        Ok(RemoteSnapshot {
            server: self.server.clone(),
            id: id.to_owned(),
            etag,
            project,
        })
    }
    pub async fn set_access(
        &self,
        id: &str,
        principal: &str,
        role: Option<&str>,
    ) -> Result<(), ShareError> {
        if principal.is_empty()
            || principal.len() > 128
            || !principal
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.@".contains(&b))
        {
            return Err(message("Enter a valid member ID"));
        }
        if role.is_some_and(|r| !matches!(r, "reader" | "writer")) {
            return Err(message("Choose reader or writer access"));
        }
        let path = format!("{}/members/{principal}", self.project_path(id)?);
        let body = role.map(|role| serde_json::json!({"role":role}).to_string());
        self.request(
            if role.is_some() { "PUT" } else { "DELETE" },
            &path,
            None,
            body.as_deref(),
        )
        .await?
        .expect(204)?;
        Ok(())
    }
}
