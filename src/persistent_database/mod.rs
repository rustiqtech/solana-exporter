use crate::persistent_database::metadata::Metadata;
use crate::SOLANA_EXPORTER_VERSION;
use log::warn;
use std::path::Path;
use std::str::FromStr;

pub mod metadata;

/// Name of database name
pub const DATABASE_NAME: &str = "persistent.db";

/// A persistent database used for storing data across `solana-exporter` runs.
/// Note: The databases will be kept backwards-compatible according to semantic version for
/// `solana-exporter`.
pub struct PersistentDatabase {
    database: sled::Db,
    metadata: Metadata,
}

impl PersistentDatabase {
    /// Creates/opens a new persistent database in the path provided.
    pub fn new(dir: &Path) -> anyhow::Result<Self> {
        let database = sled::open(dir.join(DATABASE_NAME))?;
        let metadata = Metadata::new(database.open_tree("metadata")?)?;

        let created_version = metadata.created_version()?;
        let current_version = semver::Version::from_str(SOLANA_EXPORTER_VERSION)?;

        // Semver: if major versions do not match, then they are incompatible!
        if created_version.major != current_version.major {
            warn!(
                "Database was created with exporter version {}, but the current version is {}",
                created_version, current_version
            );
        }

        Ok(Self { database, metadata })
    }

    /// Opens a tree in the database with the given name.
    pub fn tree(&self, name: &str) -> sled::Result<sled::Tree> {
        self.database.open_tree(name)
    }

    /// Returns metadata for the database.
    pub fn metadata(&self) -> &Metadata {
        &self.metadata
    }
}
