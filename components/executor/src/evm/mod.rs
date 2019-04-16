pub mod executor;

pub use executor::EVMExecutor;

use std::sync::Arc;

use cita_vm::BlockDataProvider;
use ethereum_types::{H256, U256};
use futures::future::Future;

use core_storage::Storage;

pub struct EVMBlockDataProvider<S> {
    storage: Arc<S>,
}

impl<S> EVMBlockDataProvider<S>
where
    S: Storage,
{
    pub fn new(storage: Arc<S>) -> Self {
        EVMBlockDataProvider { storage }
    }
}

impl<S> BlockDataProvider for EVMBlockDataProvider<S>
where
    S: Storage,
{
    fn get_block_hash(&self, number: &U256) -> H256 {
        let height = number.as_u64();
        let block = self
            .storage
            .get_block_by_height(height)
            .wait()
            .expect("failed to get block");

        H256::from(block.header.prevhash.into_fixed_bytes())
    }
}
