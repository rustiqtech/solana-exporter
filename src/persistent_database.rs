use std::path::Path;

/// Name of database name
pub const DATABASE_NAME: &str = "persistent.db";

/// A persistent database used for storing data across `solana-exporter` runs.
/// Note: The databases will be kept backwards-compatible according to semantic version for
/// `solana-exporter`.
pub struct PersistentDatabase {
    database: sled::Db,
}

impl PersistentDatabase {
    /// Creates/opens a new persistent database in the path provided.
    pub fn new(dir: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            database: sled::open(dir.join(DATABASE_NAME))?,
        })
    }

    /// Opens a tree in the database with the given name.
    pub fn tree(&self, name: &str) -> sled::Result<sled::Tree> {
        self.database.open_tree(name)
    }
}
