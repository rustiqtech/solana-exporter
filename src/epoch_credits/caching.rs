use anyhow::Context;
use serde::{Deserialize, Serialize};
use solana_sdk::clock::Epoch;
use std::collections::HashMap;

/// Name of the caching database.
pub const CREDIT_DB_CACHE_TREE_NAME: &str = "epoch_credit_cache";

/// A caching database for vote accounts' credit growth
pub struct EpochCreditCache {
    db: sled::Tree,
}

impl EpochCreditCache {
    /// Creates a new cache using a tree.
    pub fn new(tree: sled::Tree) -> Self {
        Self { db: tree }
    }

    /// Adds a set of a vote pubkey's epoch credits to the caching database. Returns the current
    /// set of credit history.
    pub fn add_epoch(
        &self,
        vote_pubkey: &str,
        epoch_credits: &[(Epoch, u64, u64)],
    ) -> anyhow::Result<CreditHistory> {
        let mut content = self.get_credit_history(vote_pubkey)?.unwrap_or_default();

        for (epoch, credits, prev_credits) in epoch_credits {
            let credits_info = CreditsInfo::new(*credits, *prev_credits);
            // Make sure we do not over-write an existing key.
            content.history.entry(*epoch).or_insert(credits_info);
        }

        // Drop the old value
        let _ = self.write_credit_history(vote_pubkey, &content)?;

        Ok(content)
    }

    /// Gets a vote pubkey's credit history.
    pub fn get_credit_history(&self, vote_pubkey: &str) -> anyhow::Result<Option<CreditHistory>> {
        self.db
            .get(vote_pubkey)
            .context("could not fetch from database")?
            .map(|x| bincode::deserialize(&x))
            .transpose()
            .context("could not deserialize the fetched credit_history")
    }

    /// Write a key-value pair to the database. Returns the previously inserted value, if it exists.
    fn write_credit_history(
        &self,
        vote_pubkey: &str,
        credit_history: &CreditHistory,
    ) -> anyhow::Result<Option<CreditHistory>> {
        self.db
            .insert(vote_pubkey, bincode::serialize(credit_history)?)
            .context("could not insert into database")?
            .map(|x| bincode::deserialize(&x))
            .transpose()
            .context("could not deserialize the previously inserted credit_history")
    }
}

/// The value (in key-value) for the epoch credit caching database.
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct CreditHistory {
    history: HashMap<Epoch, CreditsInfo>,
}

/// Credit information about a pubkey at a particular epoch.
#[derive(Copy, Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct CreditsInfo {
    credits: u64,
    prev_credits: u64,
}

impl CreditsInfo {
    /// Creates a new `CreditInfo`.
    pub fn new(credits: u64, prev_credits: u64) -> Self {
        CreditsInfo {
            credits,
            prev_credits,
        }
    }
}
