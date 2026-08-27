use std::sync::Arc;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::client::Inner;
use crate::error::Result;
use crate::params;

/// The `assets` resource (a publication's image library). Reached through
/// [`Mailtea::assets`](crate::Mailtea::assets).
///
/// An email or site image needs an absolute URL, so this is how a picture that
/// is not already in the library gets into one. Pointing an image at a host you
/// do not control breaks the day that host moves the file.
///
/// PNG, JPEG, GIF, WebP or SVG, 5 MB per image. The bytes are checked against
/// the declared `content_type`, so a mislabelled file is rejected rather than
/// stored.
#[derive(Clone, Debug)]
pub struct Assets {
    inner: Arc<Inner>,
}

impl Assets {
    pub(crate) fn new(inner: Arc<Inner>) -> Self {
        Self { inner }
    }

    /// `POST /v1/assets` — upload an image.
    ///
    /// ```no_run
    /// # async fn run(mailtea: mailtea::Mailtea) -> mailtea::Result<()> {
    /// use mailtea::UploadAsset;
    ///
    /// let bytes = std::fs::read("hero.png").unwrap();
    /// let asset = mailtea
    ///     .assets
    ///     .upload(&UploadAsset::from_bytes("pub_123", "hero.png", "image/png", bytes))
    ///     .await?;
    ///
    /// println!("{}", asset["url"]); // use as an image block's src
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call("POST", "/v1/assets", crate::params::to_body(params)?)
            .await
    }

    /// `GET /v1/assets` — the library, newest first. Filters:
    /// `publication_id` (required), `search` (file name), `limit` (1-200,
    /// default 100).
    pub async fn list(&self, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "GET",
                &format!("/v1/assets{}", params::query(params)?),
                None,
            )
            .await
    }

    /// `DELETE /v1/assets/:id` — retire an asset.
    ///
    /// The stored file is KEPT and its URL keeps resolving, so images inside
    /// already-sent emails do not break. This hides the asset from the library —
    /// it does not remove it from any email, template or page referencing it.
    pub async fn delete(&self, id: &str, params: impl Serialize) -> Result<Value> {
        self.inner
            .call(
                "DELETE",
                &format!(
                    "/v1/assets/{}{}",
                    crate::params::encode(id),
                    crate::params::query(params)?
                ),
                None,
            )
            .await
    }
}

/// The payload for [`Assets::upload`]. `content` is base64 —
/// [`UploadAsset::from_bytes`] does the encoding for you.
#[derive(Clone, Debug, Default, Serialize)]
pub struct UploadAsset {
    pub publication_id: String,
    pub filename: String,
    pub content_type: String,
    /// Base64-encoded bytes.
    pub content: String,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl UploadAsset {
    /// From bytes you already hold.
    pub fn from_bytes(
        publication_id: impl Into<String>,
        filename: impl Into<String>,
        content_type: impl Into<String>,
        content: impl AsRef<[u8]>,
    ) -> Self {
        use base64::Engine as _;
        Self {
            publication_id: publication_id.into(),
            filename: filename.into(),
            content_type: content_type.into(),
            content: base64::engine::general_purpose::STANDARD.encode(content),
            extra: Map::new(),
        }
    }

    /// From content you have already base64-encoded.
    pub fn from_base64(
        publication_id: impl Into<String>,
        filename: impl Into<String>,
        content_type: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            publication_id: publication_id.into(),
            filename: filename.into(),
            content_type: content_type.into(),
            content: content.into(),
            extra: Map::new(),
        }
    }
}
