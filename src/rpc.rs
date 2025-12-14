use crate::crypto::Hash;
use crate::storage::{ConsensusState, Storage};
use crate::types::{Block, Transaction, Address, U256};
use crate::tx_pool::TxPool;
use jsonrpsee::core::{RpcResult, async_trait};
use jsonrpsee::proc_macros::rpc;
use std::sync::Arc;

#[rpc(server)]
pub trait OckhamRpc {
    #[method(name = "get_block_by_hash")]
    fn get_block_by_hash(&self, hash: Hash) -> RpcResult<Option<Block>>;

    #[method(name = "get_latest_block")]
    fn get_latest_block(&self) -> RpcResult<Option<Block>>;

    #[method(name = "get_status")]
    fn get_status(&self) -> RpcResult<Option<ConsensusState>>;

    #[method(name = "send_transaction")]
    fn send_transaction(&self, tx: Transaction) -> RpcResult<Hash>;

    #[method(name = "get_balance")]
    fn get_balance(&self, address: Address) -> RpcResult<U256>;

    #[method(name = "chain_id")]
    fn chain_id(&self) -> RpcResult<u64>;
}

pub struct OckhamRpcImpl {
    storage: Arc<dyn Storage>,
    tx_pool: Arc<TxPool>,
}

impl OckhamRpcImpl {
    pub fn new(storage: Arc<dyn Storage>, tx_pool: Arc<TxPool>) -> Self {
        Self { storage, tx_pool }
    }
}

#[async_trait]
impl OckhamRpcServer for OckhamRpcImpl {
    fn get_block_by_hash(&self, hash: Hash) -> RpcResult<Option<Block>> {
        let block = self.storage.get_block(&hash).map_err(|e| {
            jsonrpsee::types::ErrorObject::owned(
                -32000,
                format!("Storage error: {:?}", e),
                None::<()>,
            )
        })?;
        Ok(block)
    }

    fn get_latest_block(&self) -> RpcResult<Option<Block>> {
        let state = self.storage.get_consensus_state().map_err(|e| {
            jsonrpsee::types::ErrorObject::owned(
                -32000,
                format!("Storage error: {:?}", e),
                None::<()>,
            )
        })?;

        if let Some(s) = state {
            let block = self.storage.get_block(&s.preferred_block).map_err(|e| {
                jsonrpsee::types::ErrorObject::owned(
                    -32000,
                    format!("Storage error: {:?}", e),
                    None::<()>,
                )
            })?;
            Ok(block)
        } else {
            Ok(None)
        }
    }

    fn get_status(&self) -> RpcResult<Option<ConsensusState>> {
        let state = self.storage.get_consensus_state().map_err(|e| {
            jsonrpsee::types::ErrorObject::owned(
                -32000,
                format!("Storage error: {:?}", e),
                None::<()>,
            )
        })?;
        Ok(state)
    }

    fn send_transaction(&self, tx: Transaction) -> RpcResult<Hash> {
        let hash = crate::crypto::hash_data(&tx);
        // Validate? (TxPool does some validation)
        self.tx_pool.add_transaction(tx).map_err(|e| {
            jsonrpsee::types::ErrorObject::owned(
                -32000,
                format!("TxPool error: {:?}", e),
                None::<()>,
            )
        })?;
        Ok(hash)
    }

    fn get_balance(&self, address: Address) -> RpcResult<U256> {
        let account = self.storage.get_account(&address).map_err(|e| {
            jsonrpsee::types::ErrorObject::owned(
                -32000,
                format!("Storage error: {:?}", e),
                None::<()>,
            )
        })?;
        
        Ok(account.map(|a| a.balance).unwrap_or_default())
    }

    fn chain_id(&self) -> RpcResult<u64> {
        Ok(1337) // TODO: Config
    }
}
