//! Storage abstraction layer for the ChainSettle contract.
//!.
        };
        use proptest::prelude::*;
        use soroban_sdk::{testutils::Address as _, token, vec, Address, BytesN, Env, String};

        fn make_test_env() -> (Env, Address, Address, Address, Address, Address, Address) {
            let env = Env::default();
            env.mock_all_auths();
            let contract_id = env.register(ChainSettleContract, ());
            let token_admin = Address::generate(&env);
            let token_id = env
                .register_stellar_asset_contract_v2(token_admin.clone())
                .address();
            let buyer = Address::generate(&env);
            let supplier = Address::generate(&env);
            let logistics = Address::generate(&env);
            let arbiter = Address::generate(&env);
            token::StellarAssetClient::new(&env, &token_id).mint(&buyer, &100_000_000_000i128);
            ChainSettleContractClient::new(&env, &contract_id).init(&buyer);
            (
                env,
                contract_id,
                token_id,
                buyer,
                supplier,
                logistics,
                arbiter,
            )
        }

        fn default_options(env: &Env) -> ShipmentOptions {
            ShipmentOptions {
                response_deadline: 0,
                penalty_bps: 0,
                milestone_mode: MilestoneMode::Parallel,
                holdback_ledgers: 0,
                dispute_cooldown_ledgers: 0,
                late_penalty_bps_per_ledger: 0,
                auto_confirm_ledgers: 0,
                dispute_bond_amount: 0,
                dispute_bond_bps: 0,
                arbiter_fee_bps: 0,
                logistics_fee_bps: 0,
                supplier_collateral: 0,
                expires_at_ledger: None,
                metadata_hash: None,
                referrer: None,
                buyer_cancel_fee_bps: 0,
                early_bonus_pool: 0,
                review_window_ledgers: None,
                milestone_splits: vec![env],
                deadlines: vec![env],
                dispute_timeout_seconds: 0,
                default_resolution: Resolution::Buyer,
                backup_arbiter: None,
                confirmation_cooldown_ledgers: None,
                arbiter_panel: vec![env],
                jurisdiction: None,
                grace_period_ledgers: 0,
                quality_grades: vec![env],
                milestone_quantities: vec![env],
                retainage_bps: 0,
                warranty_bps: 0,
                warranty_ledgers: 0,
                inspector: None,
                inspected_milestones: vec![env],
                proof_submitters: vec![env],
                require_dual_attestation: false,
                fund_from_vault: false,
                milestone_suppliers: vec![env],
                refund_recipient: None,
            }
        }

        fn milestone(env: &Env, name: &str, pct: u32) -> Milestone {
            Milestone {
                name: String::from_str(env, name),
                payment_percent: pct,
                proof_hash: String::from_str(env, ""),
                status: MilestoneStatus::Pending,
                release_after_ledger: 0,
                proof_submitted_ledger: None,
                dispute_opened_ledger: None,
                deadline_ledger: 0,
                penalty_bps_per_ledger: 0,
            }
        }

        proptest! {
            #![proptest_config(proptest::test_runner::Config {
                cases: 200,
                ..Default::default()
            })]

            /// Valid two-milestone splits (both >= 5, sum == 100) are accepted by the contract.
            #[test]
            fn prop_contract_accepts_valid_two_split(first in 5u32..=90u32) {
                let second = 100u32 - first;
                if second < 5 {
                    return Ok(());
                }
                let (env, cid, tid, buyer, supplier, logistics, arbiter) = make_test_env();
                let client = ChainSettleContractClient::new(&env, &cid);
                let sid = String::from_str(&env, "PCT-VALID");
                let ms = vec![&env, milestone(&env, "M0", first), milestone(&env, "M1", second)];
                let result = client.try_create_shipment(
                    &sid,
                    &vec![&env, buyer.clone()],
                    &supplier, &logistics, &arbiter, &tid,
                    &1_000_000i128,
                    &ms,
                    &default_options(&env),
                );
                prop_assert!(
                    result.is_ok(),
                    "valid split ({}, {}) must be accepted", first, second
                );
            }

            /// Any milestone percentage below 5 (the default minimum) is rejected.
            #[test]
            fn prop_contract_rejects_below_min_percent(below in 1u32..=4u32) {
                let (env, cid, tid, buyer, supplier, logistics, arbiter) = make_test_env();
                let client = ChainSettleContractClient::new(&env, &cid);
                let sid = String::from_str(&env, "PCT-LOW");
                let above = 100 - below;
                let ms = vec![&env, milestone(&env, "M0", below), milestone(&env, "M1", above)];
                let result = client.try_create_shipment(
                    &sid,
                    &vec![&env, buyer.clone()],
                    &supplier, &logistics, &arbiter, &tid,
                    &1_000_000i128,
                    &ms,
                    &default_options(&env),
                );
                prop_assert!(
                    result.is_err(),
                    "percent {} below minimum must be rejected", below
                );
            }

            /// Two-milestone sets whose percentages sum to something other than 100 are rejected.
            #[test]
            fn prop_contract_rejects_non_100_sum(
                first in 5u32..=45u32,
                second in 5u32..=45u32,
            ) {
                let sum = first + second;
                if sum == 100 { return Ok(()); }
                let (env, cid, tid, buyer, supplier, logistics, arbiter) = make_test_env();
                let client = ChainSettleContractClient::new(&env, &cid);
                let sid = String::from_str(&env, "PCT-BADSUM");
                let ms = vec![&env, milestone(&env, "M0", first), milestone(&env, "M1", second)];
                let result = client.try_create_shipment(
                    &sid,
                    &vec![&env, buyer.clone()],
                    &supplier, &logistics, &arbiter, &tid,
                    &1_000_000i128,
                    &ms,
                    &default_options(&env),
                );
                prop_assert!(
                    result.is_err(),
                    "sum {} != 100 must be rejected", sum
                );
            }

            /// A single 100% milestone (at or above the minimum) is always valid.
            #[test]
            fn prop_contract_accepts_single_100_milestone(_seed in 0u32..10u32) {
                let (env, cid, tid, buyer, supplier, logistics, arbiter) = make_test_env();
                let client = ChainSettleContractClient::new(&env, &cid);
                let sid = String::from_str(&env, "PCT-SINGLE");
                let ms = vec![&env, milestone(&env, "M0", 100)];
                let result = client.try_create_shipment(
                    &sid,
                    &vec![&env, buyer.clone()],
                    &supplier, &logistics, &arbiter, &tid,
                    &1_000_000i128,
                    &ms,
                    &default_options(&env),
                );
                prop_assert!(result.is_ok(), "single 100% milestone must be accepted");
            }
        }
    }
}

use soroban_sdk::{contracttype, Address, Env, String, Vec};

use crate::{
    constants::{TTL_INITIAL_LEDGERS, TTL_MAX_LEDGERS},
    AuditEntry, CancelPolicy, ContractStats, DisputeEntry, FeeConfig, MultiAdminConfig,
    ReputationScore, Shipment, ShipmentStatus,
};

// ============================================================
// VERSIONED STORAGE KEYS  (#77)
// All variants are prefixed with V1 so that a future upgrade can
// introduce V2_* keys and run migrate_v1_to_v2 without clobbering
// live state.
// ============================================================

#[contracttype]
pub enum DataKey {
    V1Shipment(String),
    V1CancelPolicy(String),
    V1AllShipments,
    /// Supplier-to-shipments index.
    V1SupplierShipments(Address),
    /// Supplier reputation score.
    V1SupplierRep(Address),
    /// Buyer-to-shipments index.
    V1BuyerShipments(Address),
    V1Admin,
    /// Ledger sequence when a milestone entered ProofSubmitted state.
    V1ProofSubmittedAt(String, u32),
    /// Pending amendment proposal.
    V1Amendment(String, u32),
    /// Optional fee configuration.
    V1FeeConfig,
    /// Minimum allowed milestone payment percent.
    V1MinMilestonePercent,
    /// Maximum concurrently open disputes per shipment.
    V1MaxConcurrentDisputes,
    /// Blacklisted addresses.
    V1Blacklisted(Address),
    /// Bounded admin action log.
    V1AdminActionLog,
    /// Whitelisted token addresses.
    V1AllowedTokens,
    /// Global pause flag.
    V1Paused,
    /// Pending arbiter rotation proposal.
    V1ArbiterRotation(String),
    /// Total escrowed value for a given token.
    V1TotalEscrowed(Address),
    /// Active disputes list.
    V1ActiveDisputes,
    /// Contract-level statistics.
    V1ContractStats,
    /// Per-status index of shipment IDs.
    V1ShipmentsByStatus(ShipmentStatus),
    /// Escalation threshold in ledgers.
    V1EscalationThreshold,
    /// Maximum shipment value cap (0 = no cap).
    V1MaxShipmentValue,
    /// Circuit breaker outflow limit.
    V1CircuitBreakerLimit,
    /// Circuit breaker window in ledgers.
    V1CircuitBreakerWindow,
    /// Circuit breaker window start ledger.
    V1CircuitBreakerWindowStart,
    /// Circuit breaker window outflow amount.
    V1CircuitBreakerWindowOutflow,
    /// Multi-admin approvals tracking.
    V1AdminApprovals(String),
    /// Multi-admin configuration.
    V1MultiAdminConfig,
    /// Pending admin nominee for two-step transfer.
    V1PendingAdmin,
}

// ============================================================
// SHIPMENT ACCESSORS
// ============================================================

pub fn get_shipment(env: &Env, shipment_id: &String) -> Option<Shipment> {
    env.storage()
        .persistent()
        .get(&DataKey::V1Shipment(shipment_id.clone()))
}

pub fn set_shipment(env: &Env, shipment_id: &String, shipment: &Shipment) {
    let key = DataKey::V1Shipment(shipment_id.clone());
    env.storage().persistent().set(&key, shipment);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_INITIAL_LEDGERS, TTL_MAX_LEDGERS);
}

pub fn shipment_exists(env: &Env, shipment_id: &String) -> bool {
    env.storage()
        .persistent()
        .has(&DataKey::V1Shipment(shipment_id.clone()))
}

pub fn extend_shipment_ttl(env: &Env, shipment_id: &String) {
    let key = DataKey::V1Shipment(shipment_id.clone());
    if env.storage().persistent().has(&key) {
        env.storage()
            .persistent()
            .extend_ttl(&key, TTL_INITIAL_LEDGERS, TTL_MAX_LEDGERS);
    }
}

// ============================================================
// CANCEL POLICY ACCESSORS
// ============================================================

pub fn get_cancel_policy(env: &Env, shipment_id: &String) -> Option<CancelPolicy> {
    env.storage()
        .persistent()
        .get(&DataKey::V1CancelPolicy(shipment_id.clone()))
}

pub fn set_cancel_policy(env: &Env, shipment_id: &String, policy: &CancelPolicy) {
    env.storage()
        .persistent()
        .set(&DataKey::V1CancelPolicy(shipment_id.clone()), policy);
}

// ============================================================
// ALL SHIPMENTS LIST
// ============================================================

pub fn get_all_shipments(env: &Env) -> Vec<String> {
    env.storage()
        .persistent()
        .get(&DataKey::V1AllShipments)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn push_shipment_id(env: &Env, shipment_id: &String) {
    let mut list = get_all_shipments(env);
    list.push_back(shipment_id.clone());
    env.storage()
        .persistent()
        .set(&DataKey::V1AllShipments, &list);
}

// ============================================================
// SUPPLIER / BUYER INDEX ACCESSORS
// ============================================================

pub fn get_supplier_shipments(env: &Env, supplier: &Address) -> Vec<String> {
    env.storage()
        .persistent()
        .get(&DataKey::V1SupplierShipments(supplier.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_supplier_shipments(env: &Env, supplier: &Address, list: &Vec<String>) {
    let key = DataKey::V1SupplierShipments(supplier.clone());
    env.storage().persistent().set(&key, list);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_INITIAL_LEDGERS, TTL_MAX_LEDGERS);
}

pub fn get_supplier_reputation(env: &Env, supplier: &Address) -> ReputationScore {
    env.storage()
        .persistent()
        .get(&DataKey::V1SupplierRep(supplier.clone()))
        .unwrap_or_default()
}

pub fn set_supplier_reputation(env: &Env, supplier: &Address, score: &ReputationScore) {
    let key = DataKey::V1SupplierRep(supplier.clone());
    env.storage().persistent().set(&key, score);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_INITIAL_LEDGERS, TTL_MAX_LEDGERS);
}

pub fn get_buyer_shipments(env: &Env, buyer: &Address) -> Vec<String> {
    env.storage()
        .persistent()
        .get(&DataKey::V1BuyerShipments(buyer.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_buyer_shipments(env: &Env, buyer: &Address, list: &Vec<String>) {
    let key = DataKey::V1BuyerShipments(buyer.clone());
    env.storage().persistent().set(&key, list);
    env.storage()
        .persistent()
        .extend_ttl(&key, TTL_INITIAL_LEDGERS, TTL_MAX_LEDGERS);
}

// ============================================================
// ADMIN ACCESSORS
// ============================================================

pub fn get_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::V1Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::V1Admin, admin);
}

// ============================================================
// PROOF SUBMITTED AT
// ============================================================

pub fn get_proof_submitted_at(env: &Env, shipment_id: &String, index: u32) -> Option<u32> {
    env.storage()
        .persistent()
        .get(&DataKey::V1ProofSubmittedAt(shipment_id.clone(), index))
}

pub fn set_proof_submitted_at(env: &Env, shipment_id: &String, index: u32, ledger: u32) {
    env.storage().persistent().set(
        &DataKey::V1ProofSubmittedAt(shipment_id.clone(), index),
        &ledger,
    );
}

// ============================================================
// ACTIVE DISPUTES
// ============================================================

pub fn get_active_disputes(env: &Env) -> Vec<DisputeEntry> {
    env.storage()
        .persistent()
        .get(&DataKey::V1ActiveDisputes)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_active_disputes(env: &Env, disputes: &Vec<DisputeEntry>) {
    env.storage()
        .persistent()
        .set(&DataKey::V1ActiveDisputes, disputes);
}

// ============================================================
// TOTAL ESCROWED
// ============================================================

pub fn get_total_escrowed(env: &Env, token: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::V1TotalEscrowed(token.clone()))
        .unwrap_or(0)
}

pub fn set_total_escrowed(env: &Env, token: &Address, amount: i128) {
    env.storage()
        .persistent()
        .set(&DataKey::V1TotalEscrowed(token.clone()), &amount);
}

// ============================================================
// CONTRACT STATS
// ============================================================

pub fn get_contract_stats(env: &Env) -> ContractStats {
    env.storage()
        .instance()
        .get(&DataKey::V1ContractStats)
        .unwrap_or(ContractStats {
            total_shipments: 0,
            total_volume: 0,
            total_disputes: 0,
            completed_shipments: 0,
        })
}

pub fn set_contract_stats(env: &Env, stats: &ContractStats) {
    env.storage()
        .instance()
        .set(&DataKey::V1ContractStats, stats);
}

// ============================================================
// STATUS INDEX
// ============================================================

pub fn get_shipments_by_status(env: &Env, status: ShipmentStatus) -> Vec<String> {
    env.storage()
        .persistent()
        .get(&DataKey::V1ShipmentsByStatus(status))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_shipments_by_status(env: &Env, status: ShipmentStatus, list: &Vec<String>) {
    env.storage()
        .persistent()
        .set(&DataKey::V1ShipmentsByStatus(status), list);
}

// ============================================================
// INSTANCE STORAGE HELPERS (paused, fee config, limits, etc.)
// ============================================================

pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::V1Paused)
        .unwrap_or(false)
}

pub fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&DataKey::V1Paused, &paused);
}

pub fn is_operation_paused(env: &Env, op: soroban_sdk::Symbol) -> bool {
    let global_paused = is_paused(env);
    if global_paused {
        return true;
    }
    env.storage()
        .instance()
        .get(&crate::DataKeyExt4::OperationPaused(op))
        .unwrap_or(false)
}

pub fn set_operation_paused(env: &Env, op: soroban_sdk::Symbol, paused: bool) {
    env.storage()
        .instance()
        .set(&crate::DataKeyExt4::OperationPaused(op), &paused);
}

pub fn get_fee_config(env: &Env) -> Option<FeeConfig> {
    env.storage().instance().get(&DataKey::V1FeeConfig)
}

pub fn set_fee_config(env: &Env, config: &FeeConfig) {
    env.storage().instance().set(&DataKey::V1FeeConfig, config);
}

pub fn get_min_milestone_percent(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::V1MinMilestonePercent)
        .unwrap_or(5u32)
}

pub fn set_min_milestone_percent(env: &Env, pct: u32) {
    env.storage()
        .instance()
        .set(&DataKey::V1MinMilestonePercent, &pct);
}

pub fn get_max_concurrent_disputes(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::V1MaxConcurrentDisputes)
        .unwrap_or(1u32)
}

pub fn set_max_concurrent_disputes(env: &Env, limit: u32) {
    env.storage()
        .instance()
        .set(&DataKey::V1MaxConcurrentDisputes, &limit);
}

pub fn is_blacklisted(env: &Env, address: &Address) -> bool {
    env.storage()
        .instance()
        .get::<DataKey, soroban_sdk::BytesN<32>>(&DataKey::V1Blacklisted(address.clone()))
        .is_some()
}

pub fn set_blacklisted(env: &Env, address: &Address, reason: &soroban_sdk::BytesN<32>) {
    env.storage()
        .instance()
        .set(&DataKey::V1Blacklisted(address.clone()), reason);
}

pub fn remove_blacklisted(env: &Env, address: &Address) {
    env.storage()
        .instance()
        .remove(&DataKey::V1Blacklisted(address.clone()));
}

pub fn get_admin_action_log(env: &Env) -> Vec<AuditEntry> {
    env.storage()
        .instance()
        .get(&DataKey::V1AdminActionLog)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_admin_action_log(env: &Env, log: &Vec<AuditEntry>) {
    env.storage()
        .instance()
        .set(&DataKey::V1AdminActionLog, log);
}

pub fn get_allowed_tokens(env: &Env) -> Vec<Address> {
    env.storage()
        .instance()
        .get(&DataKey::V1AllowedTokens)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_allowed_tokens(env: &Env, tokens: &Vec<Address>) {
    env.storage()
        .instance()
        .set(&DataKey::V1AllowedTokens, tokens);
}

pub fn get_escalation_threshold(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::V1EscalationThreshold)
        .unwrap_or(0)
}

pub fn set_escalation_threshold(env: &Env, threshold: u32) {
    env.storage()
        .instance()
        .set(&DataKey::V1EscalationThreshold, &threshold);
}

pub fn get_max_shipment_value(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::V1MaxShipmentValue)
        .unwrap_or(0)
}

pub fn set_max_shipment_value(env: &Env, value: i128) {
    env.storage()
        .instance()
        .set(&DataKey::V1MaxShipmentValue, &value);
}

pub fn get_circuit_breaker_limit(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::V1CircuitBreakerLimit)
        .unwrap_or(0)
}

pub fn set_circuit_breaker_limit(env: &Env, limit: i128) {
    env.storage()
        .instance()
        .set(&DataKey::V1CircuitBreakerLimit, &limit);
}

pub fn get_circuit_breaker_window(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::V1CircuitBreakerWindow)
        .unwrap_or(0)
}

pub fn set_circuit_breaker_window(env: &Env, window: u32) {
    env.storage()
        .instance()
        .set(&DataKey::V1CircuitBreakerWindow, &window);
}

pub fn get_circuit_breaker_window_start(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::V1CircuitBreakerWindowStart)
        .unwrap_or(0)
}

pub fn set_circuit_breaker_window_start(env: &Env, start: u32) {
    env.storage()
        .instance()
        .set(&DataKey::V1CircuitBreakerWindowStart, &start);
}

pub fn get_circuit_breaker_window_outflow(env: &Env) -> i128 {
    env.storage()
        .instance()
        .get(&DataKey::V1CircuitBreakerWindowOutflow)
        .unwrap_or(0)
}

pub fn set_circuit_breaker_window_outflow(env: &Env, outflow: i128) {
    env.storage()
        .instance()
        .set(&DataKey::V1CircuitBreakerWindowOutflow, &outflow);
}

pub fn get_multi_admin_config(env: &Env) -> Option<MultiAdminConfig> {
    env.storage().instance().get(&DataKey::V1MultiAdminConfig)
}

pub fn set_multi_admin_config(env: &Env, config: &MultiAdminConfig) {
    env.storage()
        .instance()
        .set(&DataKey::V1MultiAdminConfig, config);
}

pub fn get_admin_approvals(env: &Env, action_id: &String) -> Vec<Address> {
    env.storage()
        .persistent()
        .get(&DataKey::V1AdminApprovals(action_id.clone()))
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_admin_approvals(env: &Env, action_id: &String, approvals: &Vec<Address>) {
    env.storage()
        .persistent()
        .set(&DataKey::V1AdminApprovals(action_id.clone()), approvals);
}

pub fn remove_admin_approvals(env: &Env, action_id: &String) {
    env.storage()
        .persistent()
        .remove(&DataKey::V1AdminApprovals(action_id.clone()));
}

pub fn get_pending_admin(env: &Env) -> Option<Address> {
    env.storage().instance().get(&DataKey::V1PendingAdmin)
}

pub fn set_pending_admin(env: &Env, nominee: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::V1PendingAdmin, nominee);
}

pub fn remove_pending_admin(env: &Env) {
    env.storage().instance().remove(&DataKey::V1PendingAdmin);
}

// ============================================================
// AMENDMENT / ARBITER ROTATION (temporary storage)
// ============================================================

pub fn get_amendment(
    env: &Env,
    shipment_id: &String,
    index: u32,
) -> Option<crate::AmendmentProposal> {
    env.storage()
        .temporary()
        .get(&DataKey::V1Amendment(shipment_id.clone(), index))
}

pub fn set_amendment(
    env: &Env,
    shipment_id: &String,
    index: u32,
    proposal: &crate::AmendmentProposal,
) {
    env.storage()
        .temporary()
        .set(&DataKey::V1Amendment(shipment_id.clone(), index), proposal);
}

pub fn remove_amendment(env: &Env, shipment_id: &String, index: u32) {
    env.storage()
        .temporary()
        .remove(&DataKey::V1Amendment(shipment_id.clone(), index));
}

pub fn get_arbiter_rotation(
    env: &Env,
    shipment_id: &String,
) -> Option<crate::ArbiterRotationProposal> {
    env.storage()
        .temporary()
        .get(&DataKey::V1ArbiterRotation(shipment_id.clone()))
}

pub fn set_arbiter_rotation(
    env: &Env,
    shipment_id: &String,
    proposal: &crate::ArbiterRotationProposal,
) {
    env.storage()
        .temporary()
        .set(&DataKey::V1ArbiterRotation(shipment_id.clone()), proposal);
}

pub fn remove_arbiter_rotation(env: &Env, shipment_id: &String) {
    env.storage()
        .temporary()
        .remove(&DataKey::V1ArbiterRotation(shipment_id.clone()));
}

// ============================================================
// ARBITER FEE TIERS
// ============================================================

pub fn get_arbiter_fee_tiers(env: &Env) -> Vec<(i128, u32)> {
    env.storage()
        .persistent()
        .get(&crate::DataKeyExt::ArbiterFeeTiers)
        .unwrap_or_else(|| Vec::new(env))
}

pub fn set_arbiter_fee_tiers(env: &Env, tiers: &Vec<(i128, u32)>) {
    let key = crate::DataKeyExt::ArbiterFeeTiers;
    env.storage().persistent().set(&key, tiers);
    env.storage().persistent().extend_ttl(
        &key,
        crate::constants::TTL_INITIAL_LEDGERS,
        crate::constants::TTL_MAX_LEDGERS,
    );
}

// ============================================================
// ARBITER STATS
// ============================================================

pub fn get_arbiter_stats(env: &Env, arbiter: &Address) -> crate::ArbiterStats {
    env.storage()
        .persistent()
        .get(&crate::DataKeyExt::ArbiterStats(arbiter.clone()))
        .unwrap_or_default()
}

pub fn set_arbiter_stats(env: &Env, arbiter: &Address, stats: &crate::ArbiterStats) {
    let key = crate::DataKeyExt::ArbiterStats(arbiter.clone());
    env.storage().persistent().set(&key, stats);
    env.storage().persistent().extend_ttl(
        &key,
        crate::constants::TTL_INITIAL_LEDGERS,
        crate::constants::TTL_MAX_LEDGERS,
    );
}

// ============================================================
// STORAGE PRUNING FOR SHIPMENT FINALIZATION
// ============================================================

/// Prune auxiliary storage keys that are no longer needed after a shipment
/// reaches a terminal state (Completed, Cancelled, or Expired).
///
/// This reclaims storage rent by removing:
/// - Temporary keys (amendments, arbiter rotation proposals)
/// - Ephemeral tracking (proof submitted ledgers, dispute timestamps)
/// - Pending requests/proposals that are now moot
///
/// Should be called by shipment finalization logic after status is set to a terminal state.
pub fn prune_shipment_auxiliary_keys(env: &Env, shipment_id: &String, milestone_count: u32) {
    // Remove temporary proposal keys
    remove_arbiter_rotation(env, shipment_id);
    
    // Remove per-milestone auxiliary keys
    for idx in 0..milestone_count {
        // Remove amendment proposals
        remove_amendment(env, shipment_id, idx);
        
        // Remove proof submission timestamp tracking
        env.storage()
            .persistent()
            .remove(&DataKey::V1ProofSubmittedAt(shipment_id.clone(), idx));
    }
    
    // Remove extended keys (DataKeyExt)
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt::MilestoneSplits(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt::MilestoneTimestampDeadlines(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt::ShipmentFeeBps(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt::BackupArbiter(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt::ShipmentConfirmationCooldown(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt::DisputeDecayStartLedger(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt::ArbiterPanel(shipment_id.clone()));
    
    // Remove per-milestone extended keys
    for idx in 0..milestone_count {
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt::MilestoneInvoiceHash(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt::AmendmentLog(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt::ExtensionRequest(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt::MilestoneDeadline(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt::DisputeOpenedAt(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt::EffectiveDisputeBond(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt::DisputeVotes(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt::MilestonePayees(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt::ProofSubmitter(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt::MilestoneNotes(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt::DisputeEvidence(shipment_id.clone(), idx));
    }
    
    // Remove DataKeyExt2 keys
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt2::CoBuyer(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt2::ComplianceHold(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt2::ShipmentMediator(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt2::ShipmentPaused(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt2::ShipmentPauseRequest(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt2::ShipmentPausedAt(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt2::QualityGrades(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt2::MilestoneQuantities(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt2::RetainageBps(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt2::WarrantyConfig(shipment_id.clone()));
    
    for idx in 0..milestone_count {
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt2::JointConfirmation(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt2::DisputeResolvedAtLedger(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt2::DisputeAppealed(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt2::MediationProposal(shipment_id.clone(), idx));
    }
    
    // Remove DataKeyExt3 keys
    for idx in 0..milestone_count {
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt3::MilestoneGrade(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt3::PendingGrade(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt3::GradeDispute(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt3::DeliveredQuantity(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt3::PartialQtyReleased(shipment_id.clone(), idx));
    }
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt3::IncrementalCollateral(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt3::IncrementalCollateralBase(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt3::MilestoneMergeProposal(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt3::MilestoneSplitProposal(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt3::ShipmentInspector(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt3::InspectedMilestones(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt3::ProofSubmitters(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt3::RequireDualAttestation(shipment_id.clone()));
    
    for idx in 0..milestone_count {
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt3::InspectionReport(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt3::PendingDualProof(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt3::ConditionBreachReports(shipment_id.clone(), idx));
    }
    
    // Remove DataKeyExt4 keys
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::ShipmentFeesPaid(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::ShipmentHadDispute(shipment_id.clone()));
    
    for idx in 0..milestone_count {
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt4::SubstituteProposal(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt4::MilestoneSupplier(shipment_id.clone(), idx));
    }
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::PendingExpiryExtension(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::TotalExpiryExtended(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::CollateralToken(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::FundFromVault(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::PayoutAssignment(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::LoserPaysArbiterFee(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::CompletedAtLedger(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::RefundRecipient(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::IsConsortium(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::CollateralSlashBpsPerMiss(shipment_id.clone()));
    
    for idx in 0..milestone_count {
        env.storage()
            .persistent()
            .remove(&crate::DataKeyExt4::CollateralSlashedForMiss(shipment_id.clone(), idx));
    }
    
    env.storage()
        .persistent()
        .remove(&crate::DataKeyExt4::NewMilestoneProposal(shipment_id.clone()));
    
    // Remove DataKey keys that are temporary
    env.storage()
        .persistent()
        .remove(&DataKey::ConfirmationDelegate(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&DataKey::PendingRecovery(shipment_id.clone()));
    
    env.storage()
        .persistent()
        .remove(&DataKey::SupplierCollateral(shipment_id.clone()));
    
    for idx in 0..milestone_count {
        env.storage()
            .persistent()
            .remove(&DataKey::AdvanceRequest(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&DataKey::MilestoneProofWhitelist(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&DataKey::SubmittedProofType(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&DataKey::DisputeContestedPercent(shipment_id.clone(), idx));
        
        env.storage()
            .persistent()
            .remove(&DataKey::EvidenceCount(shipment_id.clone(), idx));
    }
}

// ============================================================
// MIGRATION STUB  (#77)
// Call once after a contract upgrade that introduces V2_* keys.
// Currently a no-op; add data-model transformation logic here.
// ============================================================

/// Migrate persistent state from V1 schema to V2 schema.
///
/// Pattern:
/// 1. Read each V1_* key.
/// 2. Transform the value if the data model changed.
/// 3. Write to the corresponding V2_* key.
/// 4. Optionally remove the V1_* key to reclaim storage rent.
///
/// This function is idempotent — safe to call multiple times.
pub fn migrate_v1_to_v2(_env: &Env) {
    // No-op for current version.
    // Example future implementation:
    //
    // let ids = get_all_shipments(env);
    // for i in 0..ids.len() {
    //     let id = ids.get(i).unwrap();
    //     if let Some(v1) = get_shipment(env, &id) {
    //         let v2 = transform_shipment_v1_to_v2(v1);
    //         env.storage().persistent().set(&DataKey::V2Shipment(id.clone()), &v2);
    //         env.storage().persistent().remove(&DataKey::V1Shipment(id));
    //     }
    // }
}
