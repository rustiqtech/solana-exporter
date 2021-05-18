use prometheus_exporter::prometheus::Error as PrometheusError;
use prometheus_exporter::prometheus::{
    register_gauge_vec, register_int_gauge, register_int_gauge_vec, GaugeVec, IntGauge, IntGaugeVec,
};
use solana_client::rpc_response::RpcVoteAccountStatus;
use solana_sdk::epoch_info::EpochInfo;

pub const PUBKEY_LABEL: &str = "pubkey";

pub struct PrometheusGauges {
    pub active_validators: IntGaugeVec,
    pub is_delinquent: GaugeVec,
    pub activated_stake: IntGaugeVec,
    pub last_vote: IntGaugeVec,
    pub root_slot: IntGaugeVec,
    pub transaction_count: IntGauge,
    pub slot_height: IntGauge,
    pub current_epoch: IntGauge,
    pub current_epoch_first_slot: IntGauge,
    pub current_epoch_last_slot: IntGauge,
    pub leader_slots: IntGaugeVec,
}

impl Default for PrometheusGauges {
    fn default() -> Self {
        Self {
            active_validators: register_int_gauge_vec!(
                "solana_active_validators",
                "Total number of active validators",
                &["state"]
            )
            .unwrap(),
            is_delinquent: register_gauge_vec!(
                "solana_validator_delinquent",
                "Whether a validator is delinquent",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            activated_stake: register_int_gauge_vec!(
                "solana_validator_activated_stake",
                "Activated stake of a validator",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            last_vote: register_int_gauge_vec!(
                "solana_validator_last_vote",
                "Last voted slot of a validator",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            root_slot: register_int_gauge_vec!(
                "solana_validator_last_vote",
                "Last voted slot of a validator",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
            transaction_count: register_int_gauge!(
                "solana_transaction_count",
                "Total number of confirmed transactions since genesis"
            )
            .unwrap(),
            slot_height: register_int_gauge!("solana_slot_height", "Last confirmed slot height")
                .unwrap(),
            current_epoch: register_int_gauge!("solana_current_epoch", "Current epoch").unwrap(),
            current_epoch_first_slot: register_int_gauge!(
                "solana_current_epoch_first_slot",
                "Current epoch's first slot"
            )
            .unwrap(),
            current_epoch_last_slot: register_int_gauge!(
                "solana_current_epoch_last_slot",
                "Current epoch's last slot"
            )
            .unwrap(),
            leader_slots: register_int_gauge_vec!(
                "solana_leader_slots",
                "Leader slots per validator ordered by skip rate",
                &[PUBKEY_LABEL]
            )
            .unwrap(),
        }
    }
}

impl PrometheusGauges {
    pub fn export_vote_accounts(
        &self,
        vote_accounts: &RpcVoteAccountStatus,
    ) -> Result<(), PrometheusError> {
        self.active_validators
            .get_metric_with_label_values(&["current"])
            .map(|m| m.set(vote_accounts.current.len() as i64))?;

        self.active_validators
            .get_metric_with_label_values(&["delinquent"])
            .map(|m| m.set(vote_accounts.delinquent.len() as i64))?;

        for v in &vote_accounts.current {
            self.is_delinquent
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(0.))?;
        }
        for v in &vote_accounts.delinquent {
            self.is_delinquent
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(1.))?;
        }
        for v in vote_accounts
            .current
            .iter()
            .chain(vote_accounts.delinquent.iter())
        {
            self.activated_stake
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(v.activated_stake as i64))?;
            self.last_vote
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(v.last_vote as i64))?;
            self.root_slot
                .get_metric_with_label_values(&[&*v.vote_pubkey])
                .map(|m| m.set(v.root_slot as i64))?;
        }

        Ok(())
    }

    pub fn export_epoch_info(&self, epoch_info: &EpochInfo) -> Result<(), PrometheusError> {
        let first_slot = epoch_info.absolute_slot as i64;
        let last_slot = first_slot + epoch_info.slots_in_epoch as i64;

        self.transaction_count
            .set(epoch_info.transaction_count.unwrap_or_default() as i64);
        self.slot_height.set(epoch_info.absolute_slot as i64);
        self.current_epoch.set(epoch_info.epoch as i64);
        self.current_epoch_first_slot.set(first_slot);
        self.current_epoch_last_slot.set(last_slot);

        Ok(())
    }
}
