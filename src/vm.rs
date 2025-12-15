use crate::crypto::Hash;
use crate::state::StateManager;
use crate::types::Block;
use revm::{
    EVM,
    primitives::{Address, CreateScheme, ExecutionResult, ResultAndState, TransactTo},
};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExecutionError {
    #[error("EVM Error: {0}")]
    Evm(String),
    #[error("State Error: {0}")]
    State(String),
    #[error("Transaction Error: {0}")]
    Transaction(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemStorage;

    #[test]
    fn test_execute_block_gas_limit() {
        let storage = Arc::new(MemStorage::new());
        let state = Arc::new(Mutex::new(StateManager::new(storage)));
        let _executor = Executor::new(state, 10_000_000); // reduced limit

        // ...
    }
}

#[derive(Clone)]
pub struct Executor {
    pub state: Arc<Mutex<StateManager>>,
    pub block_gas_limit: u64,
}

impl Executor {
    pub fn new(state: Arc<Mutex<StateManager>>, block_gas_limit: u64) -> Self {
        Self {
            state,
            block_gas_limit,
        }
    }

    pub fn execute_block(&self, block: &mut Block) -> Result<(), ExecutionError> {
        // Validation: Ensure block gas limit is respected by consensus
        // Also consensus ensures parent hash linkage.

        let mut db = self.state.lock().unwrap();
        let mut cumulative_gas_used = 0u64;
        log::info!(
            "Executing block view {} with {} txs",
            block.view,
            block.payload.len()
        );

        for tx in &block.payload {
            if tx.gas_limit > self.block_gas_limit {
                return Err(ExecutionError::Transaction(
                    "Tx exceeds block gas limit".into(),
                ));
            }
        }

        let mut receipts = Vec::with_capacity(block.payload.len());

        for (i, tx) in block.payload.iter().enumerate() {
            // 1. Validate signature (simple check here, or assume consensus did it?)
            if tx.sender() == Address::ZERO {
                return Err(ExecutionError::Transaction("Invalid sender".into()));
            }

            // 2. Setup EVM
            let mut evm = EVM::new();
            evm.database(&mut *db);

            // Set Block Info
            evm.env.block.basefee = block.base_fee_per_gas;

            // 3. Populate TxEnv
            let tx_env = &mut evm.env.tx;
            tx_env.caller = tx.sender();
            tx_env.transact_to = if let Some(to) = tx.to {
                TransactTo::Call(to)
            } else {
                TransactTo::Create(CreateScheme::Create)
            };
            tx_env.data = tx.data.clone();
            tx_env.value = tx.value;
            tx_env.gas_limit = tx.gas_limit;
            tx_env.gas_price = tx.max_fee_per_gas;
            tx_env.gas_priority_fee = Some(tx.max_priority_fee_per_gas);
            tx_env.nonce = Some(tx.nonce);

            // 4. Execute
            let result_and_state = evm
                .transact()
                .map_err(|e| ExecutionError::Evm(format!("{:?}", e)))?;

            // 5. Commit state changes
            let ResultAndState { result, state } = result_and_state;

            // Track gas and extract logs
            let (gas_used, status, logs) = match result {
                ExecutionResult::Success { gas_used, logs, .. } => (gas_used, 1u8, logs),
                ExecutionResult::Revert { gas_used, .. } => (gas_used, 0u8, vec![]),
                ExecutionResult::Halt { gas_used, .. } => (gas_used, 0u8, vec![]),
            };
            cumulative_gas_used += gas_used;
            log::info!(
                "Tx {} executed. Gas used: {}. Cumulative: {}",
                i,
                gas_used,
                cumulative_gas_used
            );

            // Create Receipt
            let receipt_logs: Vec<crate::types::Log> = logs
                .into_iter()
                .map(|l| crate::types::Log {
                    address: l.address,
                    topics: l.topics.into_iter().map(|t| Hash(t.0)).collect(),
                    data: l.data,
                })
                .collect();

            receipts.push(crate::types::Receipt {
                status,
                cumulative_gas_used,
                logs: receipt_logs,
            });

            if status == 1 {
                // Success
                for (address, account) in state {
                    let info = crate::storage::AccountInfo {
                        nonce: account.info.nonce,
                        balance: account.info.balance,
                        code_hash: Hash(account.info.code_hash.0),
                        code: account.info.code.map(|c| c.original_bytes()),
                    };

                    db.commit_account(address, info)
                        .map_err(|e| ExecutionError::State(e.to_string()))?;

                    for (index, slot) in account.storage {
                        let val = slot.present_value;
                        db.commit_storage(address, index, val)
                            .map_err(|e| ExecutionError::State(e.to_string()))?;
                    }
                }
            }
        }

        // 6. Update State Root and Gas Used in Block
        block.state_root = db.root();
        block.receipts_root = crate::types::calculate_receipts_root(&receipts);
        block.gas_used = cumulative_gas_used;
        log::info!(
            "Block Execution Complete. State Root: {:?}, Receipts Root: {:?}, Gas Used: {}",
            block.state_root,
            block.receipts_root,
            block.gas_used
        );

        Ok(())
    }
}
