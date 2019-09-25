use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;

use protocol::traits::{
    ConsensusAdapter, Context, Gossip, MemPool, MemPoolAdapter, MessageTarget, MixedTxHashes,
    Priority, Storage, StorageAdapter,
};
use protocol::types::{Epoch, Hash, Proof, Receipt, SignedTransaction, Validator};
use protocol::ProtocolResult;

// use crate::ConsensusError;

pub struct OverlordConsensusAdapter<
    G: Gossip,
    M: MemPool<MA>,
    S: Storage<SA>,
    MA: MemPoolAdapter,
    SA: StorageAdapter,
> {
    network:         Arc<G>,
    mempool:         Arc<M>,
    storage:         Arc<S>,
    mempool_adapter: PhantomData<MA>,
    storage_adapter: PhantomData<SA>,
}

#[async_trait]
impl<G, M, S, MA, SA> ConsensusAdapter for OverlordConsensusAdapter<G, M, S, MA, SA>
where
    G: Gossip + Sync + Send,
    M: MemPool<MA>,
    S: Storage<SA>,
    MA: MemPoolAdapter + 'static,
    SA: StorageAdapter + 'static,
{
    async fn get_txs_from_mempool(
        &self,
        ctx: Context,
        _epoch_id: u64,
        cycle_limit: u64,
    ) -> ProtocolResult<MixedTxHashes> {
        self.mempool.package(ctx, cycle_limit).await
    }

    async fn check_txs(&self, ctx: Context, check_txs: Vec<Hash>) -> ProtocolResult<()> {
        self.mempool.ensure_order_txs(ctx, check_txs).await
    }

    async fn sync_txs(&self, ctx: Context, txs: Vec<Hash>) -> ProtocolResult<()> {
        self.mempool.sync_propose_txs(ctx, txs).await
    }

    async fn get_full_txs(
        &self,
        ctx: Context,
        txs: Vec<Hash>,
    ) -> ProtocolResult<Vec<SignedTransaction>> {
        self.mempool.get_full_txs(ctx, txs).await
    }

    async fn transmit(
        &self,
        ctx: Context,
        msg: Vec<u8>,
        end: &str,
        _target: MessageTarget,
    ) -> ProtocolResult<()> {
        self.network.broadcast(ctx, end, msg, Priority::High).await
    }

    async fn execute(
        &self,
        _ctx: Context,
        _signed_txs: Vec<SignedTransaction>,
    ) -> ProtocolResult<()> {
        Ok(())
    }

    async fn flush_mempool(&self, ctx: Context, txs: Vec<Hash>) -> ProtocolResult<()> {
        self.mempool.flush(ctx, txs).await
    }

    async fn save_epoch(&self, _ctx: Context, epoch: Epoch) -> ProtocolResult<()> {
        self.storage.insert_epoch(epoch).await
    }

    async fn save_receipts(&self, _ctx: Context, receipts: Vec<Receipt>) -> ProtocolResult<()> {
        self.storage.insert_receipts(receipts).await
    }

    async fn save_proof(&self, _ctx: Context, proof: Proof) -> ProtocolResult<()> {
        self.storage.update_latest_proof(proof).await
    }

    async fn save_signed_txs(
        &self,
        _ctx: Context,
        signed_txs: Vec<SignedTransaction>,
    ) -> ProtocolResult<()> {
        self.storage.insert_transactions(signed_txs).await
    }

    async fn get_last_validators(
        &self,
        _ctx: Context,
        epoch_id: u64,
    ) -> ProtocolResult<Vec<Validator>> {
        let epoch = self.storage.get_epoch_by_epoch_id(epoch_id).await?;
        Ok(epoch.header.validators)
    }
}

impl<G, M, S, MA, SA> OverlordConsensusAdapter<G, M, S, MA, SA>
where
    G: Gossip + Sync + Send,
    M: MemPool<MA>,
    S: Storage<SA>,
    MA: MemPoolAdapter + 'static,
    SA: StorageAdapter + 'static,
{
    pub fn new(network: Arc<G>, mempool: Arc<M>, storage: Arc<S>) -> Self {
        OverlordConsensusAdapter {
            mempool_adapter: PhantomData,
            storage_adapter: PhantomData,
            network,
            mempool,
            storage,
        }
    }
}
