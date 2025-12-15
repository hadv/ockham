use crate::crypto::Hash;
use crate::types::Transaction;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PoolError {
    #[error("Transaction already exists")]
    AlreadyExists,
    #[error("Invalid signature")]
    InvalidSignature,
}

/// A simple Transaction Pool (Mempool).
/// proper implementation should handle nonce ordering and gas price sorting.
/// MVP: Simple FIFO/Map.
#[derive(Clone, Default)]
pub struct TxPool {
    // Map Hash -> Transaction for quick lookup
    transactions: Arc<Mutex<HashMap<Hash, Transaction>>>,
    // Queue for FIFO ordering (MVP)
    queue: Arc<Mutex<VecDeque<Hash>>>,
}

impl TxPool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a transaction to the pool.
    pub fn add_transaction(&self, tx: Transaction) -> Result<(), PoolError> {
        // TODO: Validate signature
        // TODO: Validate nonce against state (require access to StateManager?)

        let hash = crate::crypto::hash_data(&tx);

        let mut text_map = self.transactions.lock().unwrap();
        if text_map.contains_key(&hash) {
            return Err(PoolError::AlreadyExists);
        }

        text_map.insert(hash, tx);
        self.queue.lock().unwrap().push_back(hash);

        Ok(())
    }

    /// Get a batch of transactions for a new block, respecting the gas limit.
    /// Ordered by Gas Price (max_fee_per_gas) Descending.
    pub fn get_transactions_for_block(
        &self,
        block_gas_limit: u64,
        base_fee: crate::types::U256,
    ) -> Vec<Transaction> {
        let mut pending = Vec::new();
        let map = self.transactions.lock().unwrap();

        // 1. Collect and Filter transactions
        let mut all_txs: Vec<&Transaction> = map
            .values()
            .filter(|tx| tx.max_fee_per_gas >= base_fee)
            .collect();

        // 2. Sort by Effective Tip Descending
        // Effective Tip = min(max_priority_fee, max_fee - base_fee)
        all_txs.sort_by(|a, b| {
            let tip_a = std::cmp::min(a.max_priority_fee_per_gas, a.max_fee_per_gas - base_fee);
            let tip_b = std::cmp::min(b.max_priority_fee_per_gas, b.max_fee_per_gas - base_fee);
            tip_b.cmp(&tip_a) // Descending
        });

        // 3. Select fitting transactions
        let mut current_gas = 0u64;

        for tx in all_txs {
            if current_gas + tx.gas_limit <= block_gas_limit {
                pending.push(tx.clone());
                current_gas += tx.gas_limit;
            }
            // Optimize: If block is full, break?
            if current_gas >= block_gas_limit {
                break;
            }
        }

        pending
    }

    /// Remove transactions that were included in a block.
    pub fn remove_transactions(&self, txs: &[Transaction]) {
        let mut map = self.transactions.lock().unwrap();
        let mut queue = self.queue.lock().unwrap();

        for tx in txs {
            let hash = crate::crypto::hash_data(tx);
            if map.remove(&hash).is_some() {
                // Remove from queue is O(N). Vector might be better or LinkedHashMap.
                // For MVP, simplistic rebuild or filter.
                // Or just keep it simple.
                if let Some(pos) = queue.iter().position(|h| *h == hash) {
                    queue.remove(pos);
                }
            }
        }
    }

    pub fn len(&self) -> usize {
        self.transactions.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.transactions.lock().unwrap().is_empty()
    }
}
