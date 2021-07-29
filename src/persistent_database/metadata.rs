use crate::SOLANA_EXPORTER_VERSION;
use anyhow::{anyhow, Context};
use semver::Version;
use serde::de::DeserializeOwned;
use serde::Serialize;
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
            .get(CREATED_VERSION)
            .context("could not get created_version from database")?
            .map(|x| String::from_utf8(x.to_vec()))
            .transpose()
            .context("created_version from database is not valid UTF-8")?
            .map(|x| Version::from_str(&x))
            .transpose()
            .context("created_version from database is not valid semver")?
            .ok_or_else(|| anyhow!("no created_version in metadata"))
    }

    /// Returns the metadata struct for a particular tree.
    pub fn get_metadata<T>(&self, tree_name: &str) -> anyhow::Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        self.tree
            .get(tree_name)
            .context("could not get metadata")?
            .map(|x| bincode::deserialize(&x))
            .transpose()
            .context("could not deserialize fetched metadata")
    }

    /// Sets the metadata struct for a particular tree. Returns the previously inserted value.
    pub fn set_metadata<T>(&self, tree_name: &str, value: &T) -> anyhow::Result<Option<T>>
    where
        T: Serialize + DeserializeOwned,
    {
        self.tree
            .insert(
                tree_name,
                bincode::serialize(value).context("could not serialize metadata")?,
            )
            .context("could not insert metadata into database")?
            .map(|x| bincode::deserialize(&x))
            .transpose()
            .context("could not deserialize the previously inserted metadata value")
    }
}
