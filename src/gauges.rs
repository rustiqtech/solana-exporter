use prometheus_exporter::prometheus::{
    register_gauge_vec, register_int_gauge, register_int_gauge_vec, GaugeVec, IntGauge, IntGaugeVec,
};

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
