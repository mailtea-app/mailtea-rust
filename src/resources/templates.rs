use std::sync::Arc;

use serde::Serialize;
use serde_json::Value;

use crate::client::Inner;
use crate::error::Result;
use crate::params;

/// The `templates` resource (reusable server-side email templates). Reached
/// through [`Mailtea::templates`](crate::Mailtea::templates).
///
/// Templates are scoped to a publication — pass `publication_id` (except
/// [`render`](Templates::render), which just renders a spec). Create one from
/// raw `html`, a json-render `spec`, or an `editor_doc` (a Studio editor
/// design), then [`publish`](Templates::publish) it before seeding
/// posts/emails from it.
#[derive(Clone, Debug)]
pub struct Templates {
    inner: Arc<Inner>,
}

impl Templates {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/templates/render` — render a json-render `spec` (with optional
    /// `variables`) to HTML without creating a template. Returns
    /// `{"html": ..., "text": ...}`.
    pub async fn render(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "POST",
                "/v1/templates/render",
                crate::params::to_body(params)?,
            )
            .await
    }

    /// `POST /v1/templates` — create a template from `html`, a `spec`, OR an
    /// `editor_doc` (exactly one; the server renders `html` from an
    /// `editor_doc`, so do not send both).
    ///
    /// Takes `publication_id` and `name`, plus optional `style_profile`,
    /// `mailtea_theme`, `global_css`, `category`, `preview_image_url`, `tags`,
    /// `description`, `text`, `subject`, `from`, `reply_to` and `variables`.
    pub async fn create(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/templates", crate::params::to_body(params)?)
            .await
    }

    /// `GET /v1/templates` — list templates, cursor-paginated. Filters:
    /// `publication_id` (required), `limit`, `after`.
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/templates{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `GET /v1/templates/:id` — one template. Requires `publication_id`.
    pub async fn get(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/templates/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `PATCH /v1/templates/:id` — update a template's content or metadata.
    ///
    /// An `editor_doc` re-renders `html` server-side, so do not send both.
    /// `global_css`, `category`, `preview_image_url`, `tags`, `text`,
    /// `subject`, `from` and `reply_to` accept `null` to clear them.
    /// `publication_id` is required and is sent as a query parameter.
    ///
    /// Editing a published template no longer unpublishes it: the change is
    /// saved as the working copy, the template keeps its published status,
    /// and the published version keeps sending until
    /// [`publish`](Self::publish) is called again. The reply's `unpublished`
    /// is kept for compatibility and is always `false` now; check
    /// `has_unpublished_versions` on the reply instead (it also carries
    /// `message` when that is `true`).
    pub async fn update(&self, id: &str, params: impl Serialize) -> Result<Value> {
        let payload = crate::params::to_value(params)?;
        self.inner
            .call(
                "PATCH",
                &format!(
                    "/v1/templates/{}{}",
                    crate::params::encode(id),
                    crate::params::query_pick(&payload, &["publication_id"])
                ),
                crate::params::body_from_value(&payload)?,
            )
            .await
    }

    /// `POST /v1/templates/:id/publish` — publish a template so it can seed
    /// posts and emails. Requires `publication_id`.
    pub async fn publish(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "POST",
                &format!(
                    "/v1/templates/{}/publish{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `POST /v1/templates/:id/unpublish` — return a published template to
    /// draft. `published_at` is kept: it records that the template was published
    /// once, not that it still is. This is now the only way to stop a
    /// published template sending, short of deleting it (editing or
    /// restoring it no longer does that on its own). It also drops the
    /// published version, so the next [`publish`](Self::publish) starts from
    /// the current (working) content. Requires `publication_id`.
    pub async fn unpublish(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "POST",
                &format!(
                    "/v1/templates/{}/unpublish{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `GET /v1/templates/:id/versions` — the design history, newest first.
    /// Requires `publication_id`; optional `limit`.
    ///
    /// Entries are metadata only. The design document is never included,
    /// because one entry alone can carry half a megabyte of it. `is_current` marks the entry that
    /// matches the working copy (the saved design being edited), which is not
    /// always the newest entry: a metadata-only update touches the template
    /// without recording a version. `is_published` (a bool) marks the entry
    /// automations and the API are sending now. They differ while a published
    /// template has unpublished changes. `is_published` is `false` on every
    /// entry of a draft, and on every entry of a template published before the
    /// field existed until it is published again.
    pub async fn versions(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!(
                    "/v1/templates/{}/versions{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `POST /v1/templates/:id/versions/:version/restore` — put an older design
    /// back onto the template. Requires `publication_id`.
    ///
    /// **Restoring no longer unpublishes the template.** It is a content
    /// write, and lands in the working copy: a published template keeps its
    /// published status and keeps sending its published version until
    /// [`publish`](Self::publish) makes the restored design live. The reply's
    /// `unpublished` is kept for compatibility and is always `false` now;
    /// check `has_unpublished_versions` on the returned `template` (or the
    /// reply's `message`) to see whether the restored design is live yet.
    ///
    /// History is forward-only: the design being replaced is recorded as its own
    /// version first, then the restored design is appended as the new newest
    /// one. Restoring the design that is already current writes nothing and
    /// returns `restored: false` with `reason: "identical"`.
    pub async fn restore_version(
        &self,
        id: &str,
        version: impl std::fmt::Display,
        params: impl Serialize,
    ) -> Result<Value> {
        self.inner
            .call(
                "POST",
                &format!(
                    "/v1/templates/{}/versions/{}/restore{}",
                    crate::params::encode(id),
                    crate::params::encode(&version.to_string()),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `POST /v1/templates/:id/duplicate` — duplicate a template into a new
    /// draft. Requires `publication_id`.
    pub async fn duplicate(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "POST",
                &format!(
                    "/v1/templates/{}/duplicate{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }

    /// `DELETE /v1/templates/:id` — delete a template. Requires
    /// `publication_id`.
    pub async fn delete(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "/v1/templates/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }
}
