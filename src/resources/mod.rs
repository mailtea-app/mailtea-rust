//! One module per API resource, mirroring the Python SDK method for method and
//! path for path.

mod api_keys;
mod assets;
mod automation_runs;
mod automations;
mod contact_properties;
mod contacts;
mod domains;
mod emails;
mod events;
mod inbound;
mod posts;
mod segments;
mod senders;
mod suppressions;
mod templates;
mod topics;
mod webhooks;

pub use api_keys::ApiKeys;
pub use assets::{Assets, UploadAsset};
pub use automation_runs::AutomationRuns;
pub use automations::Automations;
pub use contact_properties::ContactProperties;
pub use contacts::{Contacts, CreateContact, UpdateContact};
pub use domains::{DomainClaims, Domains, TrackingDomains};
pub use emails::{Attachment, BatchSent, Email, Emails, SendEmail, SentEmail, Tag, TemplateRef};
pub use events::{EventDefinitions, Events};
pub use inbound::{InboundAttachments, InboundEmails};
pub use posts::{CreatePost, Posts, SendPost, SendTestPost};
pub use segments::Segments;
pub use senders::Senders;
pub use suppressions::Suppressions;
pub use templates::Templates;
pub use topics::{CreateTopic, Topics};
pub use webhooks::Webhooks;
