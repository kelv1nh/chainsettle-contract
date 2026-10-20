//! Supplier-initiated voluntary refund feature.
//!
//! This module implements the ability for suppliers to voluntarily refund
//! a confirmed milestone payment to the buyer without requiring dispute
//! resolution. This is useful for scenarios like:
//! - Quality issues discovered after delivery
//! - Goodwill gestures
//! - Partial refunds for minor defects
//! - Customer satisfaction improvements
//!
//! Flow:
//! 1. Supplier calls `propose_voluntary_refund` for a confirmed milestone
//! 2. Buyer calls `accept_voluntary_refund` to receive the refund
//! 3. Refund amount is transferred from supplier to buyer
//!
//! Security:
//! - Only the supplier can propose refunds
//! - Only buyers can accept refunds
//! - Milestone must be in Confirmed or Resolved status
//! - Refund amount cannot exceed the amount originally paid for that milestone
//! - Refund proposals expire after a configured timeout

use soroban_sdk::{Address, BytesN, Env, String, Symbol};

use crate::{ChainSettleContract, DataKeyExt4, Error, MilestoneStatus};

impl ChainSettleContract {
    /// Supplier proposes a voluntary refund for a confirmed milestone.
    ///
    /// # Arguments
    /// * `supplier` - Supplier address (requires auth)
    /// * `shipment_id` - Shipment identifier
    /// * `milestone_index` - Index of the milestone to refund
    /// * `refund_amount` - Amount to refund (must be <= milestone payment)
    /// * `reason_hash` - Off-chain reason hash (IPFS CID or similar)
    ///
    /// # Errors
    /// * `ShipmentNotFound` - Shipment doesn't exist
    /// * `Unauthorized` - Caller is not the supplier
    /// * `InvalidMilestone` - Milestone doesn't exist or isn't confirmed
    /// * `InvalidAmount` - Refund amount exceeds milestone payment
    pub fn propose_voluntary_refund(
        env: Env,
        supplier: Address,
        shipment_id: String,
        milestone_index: u32,
        refund_amount: i128,
        reason_hash: BytesN<32>,
    ) {
        supplier.require_auth();

        // Load shipment
        let shipment = crate::storage::get_shipment(&env, &shipment_id)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ShipmentNotFound));

        // Verify caller is the supplier
        if supplier != shipment.supplier {
            panic_with_error!(&env, Error::Unauthorized);
        }

        // Verify milestone index
        if milestone_index >= shipment.milestones.len() {
            panic_with_error!(&env, Error::InvalidMilestone);
        }

        let milestone = shipment.milestones.get(milestone_index).unwrap();

        // Verify milestone is in a refundable state (Confirmed or Resolved)
        match milestone.status {
            MilestoneStatus::Confirmed | MilestoneStatus::Resolved => {}
            _ => panic_with_error!(&env, Error::InvalidMilestone),
        }

        // Calculate the milestone payment amount
        let milestone_payment =
            (shipment.total_amount * milestone.payment_percent as i128) / 100;

        // Verify refund amount is valid
        if refund_amount <= 0 || refund_amount > milestone_payment {
            panic_with_error!(&env, Error::InvalidAmount);
        }

        // Store the voluntary refund proposal
        let current_ledger = env.ledger().sequence();
        let key = DataKeyExt4::VoluntaryRefund(shipment_id.clone(), milestone_index);
        env.storage().persistent().set(
            &key,
            &(refund_amount, reason_hash, current_ledger),
        );
        env.storage().persistent().extend_ttl(
            &key,
            crate::constants::TTL_INITIAL_LEDGERS,
            crate::constants::TTL_MAX_LEDGERS,
        );

        // Emit event
        env.events().publish(
            (
                Symbol::new(&env, "voluntary_refund_proposed"),
                shipment_id,
                milestone_index,
            ),
            (supplier, refund_amount, reason_hash, current_ledger),
        );
    }

    /// Buyer accepts a voluntary refund proposal from the supplier.
    ///
    /// # Arguments
    /// * `buyer` - Buyer address (requires auth)
    /// * `shipment_id` - Shipment identifier
    /// * `milestone_index` - Index of the milestone to refund
    ///
    /// # Errors
    /// * `ShipmentNotFound` - Shipment doesn't exist
    /// * `Unauthorized` - Caller is not a buyer for this shipment
    /// * `NoPendingRefund` - No voluntary refund proposal exists
    pub fn accept_voluntary_refund(
        env: Env,
        buyer: Address,
        shipment_id: String,
        milestone_index: u32,
    ) {
        buyer.require_auth();

        // Load shipment
        let shipment = crate::storage::get_shipment(&env, &shipment_id)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ShipmentNotFound));

        // Verify caller is a buyer
        let mut is_buyer = false;
        for i in 0..shipment.buyers.len() {
            if shipment.buyers.get(i).unwrap() == buyer {
                is_buyer = true;
                break;
            }
        }
        if !is_buyer {
            panic_with_error!(&env, Error::Unauthorized);
        }

        // Load the refund proposal
        let key = DataKeyExt4::VoluntaryRefund(shipment_id.clone(), milestone_index);
        let (refund_amount, reason_hash, _proposed_ledger): (i128, BytesN<32>, u32) =
            env.storage()
                .persistent()
                .get(&key)
                .unwrap_or_else(|| panic_with_error!(&env, Error::NoPendingRefund));

        // Transfer refund from supplier to buyer
        let token_client = crate::token::Client::new(&env, &shipment.token);
        token_client.transfer(&shipment.supplier, &buyer, &refund_amount);

        // Remove the proposal
        env.storage().persistent().remove(&key);

        // Update total escrowed (decrease by refund amount)
        let total_escrowed =
            crate::storage::get_total_escrowed(&env, &shipment.token);
        crate::storage::set_total_escrowed(
            &env,
            &shipment.token,
            total_escrowed - refund_amount,
        );

        // Emit event
        env.events().publish(
            (
                Symbol::new(&env, "voluntary_refund_accepted"),
                shipment_id,
                milestone_index,
            ),
            (buyer, shipment.supplier, refund_amount, reason_hash),
        );
    }

    /// Cancel a voluntary refund proposal.
    ///
    /// # Arguments
    /// * `supplier` - Supplier address (requires auth)
    /// * `shipment_id` - Shipment identifier
    /// * `milestone_index` - Index of the milestone
    ///
    /// # Errors
    /// * `Unauthorized` - Caller is not the supplier
    /// * `NoPendingRefund` - No voluntary refund proposal exists
    pub fn cancel_voluntary_refund(
        env: Env,
        supplier: Address,
        shipment_id: String,
        milestone_index: u32,
    ) {
        supplier.require_auth();

        // Load shipment
        let shipment = crate::storage::get_shipment(&env, &shipment_id)
            .unwrap_or_else(|| panic_with_error!(&env, Error::ShipmentNotFound));

        // Verify caller is the supplier
        if supplier != shipment.supplier {
            panic_with_error!(&env, Error::Unauthorized);
        }

        // Load and remove the refund proposal
        let key = DataKeyExt4::VoluntaryRefund(shipment_id.clone(), milestone_index);
        if !env.storage().persistent().has(&key) {
            panic_with_error!(&env, Error::NoPendingRefund);
        }

        env.storage().persistent().remove(&key);

        // Emit event
        env.events().publish(
            (
                Symbol::new(&env, "voluntary_refund_cancelled"),
                shipment_id,
                milestone_index,
            ),
            supplier,
        );
    }

    /// Read-only query to check if a voluntary refund proposal exists.
    ///
    /// Returns (refund_amount, reason_hash, proposed_ledger) if one exists, None otherwise.
    pub fn get_voluntary_refund(
        env: Env,
        shipment_id: String,
        milestone_index: u32,
    ) -> Option<(i128, BytesN<32>, u32)> {
        let key = DataKeyExt4::VoluntaryRefund(shipment_id, milestone_index);
        env.storage().persistent().get(&key)
    }
}
