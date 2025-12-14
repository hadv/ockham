
use crate::crypto::Hash;
use crate::state::StateManager;
use crate::types::Block;
use revm::{
    primitives::{Address, ExecutionResult, TransactTo, ResultAndState, CreateScheme},
    EVM,
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

#[derive(Clone)]
pub struct Executor {
    pub state: Arc<Mutex<StateManager>>,
}

impl Executor {
    pub fn new(state: Arc<Mutex<StateManager>>) -> Self {
        Self { state }
    }

    pub fn execute_block(&self, block: &mut Block) -> Result<(), ExecutionError> {
        // TODO: Validate block header, parent hash, etc? logic is in consensus.
        // Here we just execute transactions and update state root.
        
        let mut db = self.state.lock().unwrap();
        
        for tx in &block.payload {
            // 1. Validate signature (simple check here, or assume consensus did it?)
            // Ideally we check signatures before execution.
            if tx.sender() == Address::ZERO {
                 return Err(ExecutionError::Transaction("Invalid sender".into()));
            }

            // 2. Setup EVM
            let mut evm = EVM::new();
            evm.database(&mut *db);
            
            // 3. Populate TxEnv
            // 3. Populate TxEnv
            // evm.env matches this version of revm
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
            // EIP-1559 to Legacy mapping for this revm version
            // map max_fee_per_gas to gas_price
            tx_env.gas_price = tx.max_fee_per_gas;
            // tx_env.max_fee_per_gas = ... (removed)
            // tx_env.max_priority_fee_per_gas = ... (removed) 
            tx_env.nonce = Some(tx.nonce);

            // 4. Execute
            let result_and_state = evm.transact().map_err(|e| ExecutionError::Evm(format!("{:?}", e)))?;
            
            // 5. Commit state changes
            // result_and_state has .result (ExecutionResult) and .state (State = HashMap<Address, Account>)
            let ResultAndState { result, state } = result_and_state;
            
            if let ExecutionResult::Success { .. } = result {
                for (address, account) in state {
                     // Always update for now, or check status if needed.
                     // revm usually returns changed state in `ResultAndState`.
                     
                     // Update account info
                     let info = crate::storage::AccountInfo {
                         nonce: account.info.nonce,
                         balance: account.info.balance,
                         code_hash: Hash(account.info.code_hash.0),
                         code: account.info.code.map(|c| c.original_bytes()),
                     };
                     
                     db.commit_account(address, info).map_err(|e| ExecutionError::State(e.to_string()))?;
                     
                     // Update storage
                     for (index, slot) in account.storage {
                         // value in revm is cast to U256 (slot value). 
                         // revm 3.x storage value is U256. 
                         // But we need to check if it's present (Slot specific).
                         // In 3.0 storage is `HashMap<U256, Slot>`. Slot has `present_value`.
                         let val = slot.present_value;
                         db.commit_storage(address, index, val).map_err(|e| ExecutionError::State(e.to_string()))?;
                     }
                }
            }
            
            // TODO: Collect receipts/logs for block header
        }
        
        // 6. Update State Root in Block
        block.state_root = db.root();
        
        Ok(())
    }
}
