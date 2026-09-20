//! Installing and managing the speech models.

mod download;
mod registry;
mod store;

pub use download::{download, DownloadProgress};
pub use registry::{find, ModelSpec, RemoteFile, ALL, DEFAULT};
pub use store::{InstallState, ModelStore};
