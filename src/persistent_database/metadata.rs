use crate::SOLANA_EXPORTER_VERSION;
use anyhow::anyhow;
use semver::Version;
use std::str::FromStr;

const CREATED_VERSION: &str = "created_version";

/// A tree containing metadata information about the persistent database.
pub struct Metadata {
    tree: sled::Tree,
}

impl Metadata {
    /// Creates a new Metadata struct.
    pub fn new(tree: sled::Tree) -> sled::Result<Self> {
        // Set created_version if the key-value pair does not already exist. Since we never delete
        // the key, this is guaranteed to only run once once on creation.
        if tree.get(CREATED_VERSION)?.is_none() {
            tree.insert(CREATED_VERSION, SOLANA_EXPORTER_VERSION)?;
        }

        Ok(Self { tree })
    }

    /// Returns the `solana-exporter` version that created this database.
    pub fn created_version(&self) -> anyhow::Result<Version> {
        self.tree
            .get(CREATED_VERSION)?
            .map(|x| String::from_utf8(x.to_vec()))
            .transpose()?
            .map(|x| Version::from_str(&x))
            .transpose()?
            .ok_or_else(|| anyhow!("no created_version in metadata"))
    }
}
