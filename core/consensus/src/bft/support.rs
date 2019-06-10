use std::collections::HashMap;
use std::convert::TryInto;
use std::sync::Arc;

use bft_rs::{
    Address as BFTAddress, BftMsg, BftSupport, Commit, Node as BftNode, Signature,
    Status as BftStatus,
};
use futures::executor::block_on;
use parking_lot::RwLock;

use core_context::{Context, P2P_SESSION_ID};
use core_crypto::{Crypto, CryptoTransform};
use core_runtime::network::Consensus as Network;
use core_runtime::{ConsensusError, Executor, Storage, TransactionPool};
use core_serialization::{AsyncCodec, Proposal as SerProposal};
use core_types::{Address, Hash, Proof, Proposal, Vote};

use crate::{ConsensusResult, Engine};

#[derive(Clone)]
pub(crate) struct ProposalOriginValue {
    height:     u64,
    session_id: usize,
}

pub(crate) struct Support<E, T, S, C, N>
where
    E: Executor + 'static,
    T: TransactionPool + 'static,
    S: Storage + 'static,
    C: Crypto + 'static,
    N: Network + 'static,
{
    engine: Arc<Engine<E, T, S, C>>,

    network:         N,
    proposal_origin: RwLock<HashMap<Hash, ProposalOriginValue>>,
}

impl<E, T, S, C, N> Support<E, T, S, C, N>
where
    E: Executor + 'static,
    T: TransactionPool + 'static,
    S: Storage + 'static,
    C: Crypto + 'static,
    N: Network + 'static,
{
    pub(crate) fn new(engine: Arc<Engine<E, T, S, C>>, network: N) -> ConsensusResult<Self> {
        Ok(Self {
            engine,
            network,

            proposal_origin: RwLock::new(HashMap::new()),
        })
    }

    pub(crate) fn insert_proposal_origin(
        &self,
        hash: Hash,
        session_id: usize,
        height: u64,
    ) -> ConsensusResult<()> {
        let mut proposal_origin = self.proposal_origin.write();

        proposal_origin.insert(hash, ProposalOriginValue { height, session_id });
        Ok(())
    }

    pub(crate) fn get_proposal_origin(
        &self,
        hash: &Hash,
    ) -> ConsensusResult<Option<ProposalOriginValue>> {
        let proposal_origin = self.proposal_origin.read();
        Ok(proposal_origin.get(hash).map(Clone::clone))
    }
}

impl<E, T, S, C, N> BftSupport for Support<E, T, S, C, N>
where
    E: Executor + 'static,
    T: TransactionPool + 'static,
    S: Storage + 'static,
    C: Crypto + 'static,
    N: Network + 'static,
{
    type Error = ConsensusError;

    /// A user-defined function for block validation.
    /// Every block bft received will call this function, even if the feed
    /// block. Users should validate block format, block headers here.
    fn check_block(&self, proposal: &[u8], _: &[u8], _height: u64) -> Result<(), Self::Error> {
        let fut = async move {
            let proposal: Proposal = AsyncCodec::decode::<SerProposal>(proposal.to_vec())
                .await?
                .try_into()?;

            // Ignore the self proposal
            let status = self.engine.get_status();
            if proposal.proposer == status.node_address {
                return Ok(());
            }

            let ctx = Context::new();
            self.engine.verify_proposal(ctx.clone(), &proposal)?;
            Ok(())
        };

        block_on(fut)
    }

    /// A user-defined function for transactions validation.
    /// Every block bft received will call this function, even if the feed
    /// block. Users should validate transactions here.
    /// The [`proposal_hash`] is corresponding to the proposal of the
    /// [`proposal_hash`].
    fn check_txs(
        &self,
        proposal: &[u8],
        _: &[u8],
        signed_proposal_hash: &[u8],
        _height: u64,
        _round: u64,
    ) -> Result<(), Self::Error> {
        let fut = async move {
            let proposal: Proposal = AsyncCodec::decode::<SerProposal>(proposal.to_vec())
                .await?
                .try_into()?;

            if proposal.tx_hashes.is_empty() {
                return Ok(());
            }

            // Ignore the self proposal
            let status = self.engine.get_status();
            if proposal.proposer == status.node_address {
                return Ok(());
            }

            let hash = Hash::from_bytes(signed_proposal_hash)?;
            let value = self.get_proposal_origin(&hash)?.ok_or_else(|| {
                ConsensusError::InvalidProposal(
                    "the origin of the proposal could not be found".to_owned(),
                )
            })?;

            let ctx = Context::new().with_value(P2P_SESSION_ID, value.session_id);

            self.engine
                .verify_transactions(ctx.clone(), proposal.clone())
                .await?;
            Ok(())
        };

        block_on(fut)
    }

    /// A user-defined function for transmitting signed_proposals and
    /// signed_votes. The signed_proposals and signed_votes have been
    /// serialized, users do not have to care about the structure of
    /// Proposal and Vote.
    fn transmit(&self, msg: BftMsg) {
        let network = self.network.clone();

        match msg {
            BftMsg::Proposal(proposal) => {
                let height = self.engine.get_status().height;
                let mut proposal_with_height = proposal.clone();
                proposal_with_height.extend(height.to_be_bytes().iter().cloned());
                network.proposal(proposal_with_height);
            }
            BftMsg::Vote(vote) => network.vote(vote),
            _ => {}
        }
    }

    /// A user-defined function for processing the reaching-consensus block.
    /// Users could execute the block inside and add it into chain.
    /// The height of proof inside the commit equals to block height.
    fn commit(&self, commit: Commit) -> Result<BftStatus, Self::Error> {
        let fut = async move {
            let proposal: Proposal = AsyncCodec::decode::<SerProposal>(commit.block.to_vec())
                .await?
                .try_into()?;

            let mut commits: Vec<Vote> = Vec::with_capacity(commit.proof.precommit_votes.len());
            for (address, signature) in commit.proof.precommit_votes.into_iter() {
                commits.push(Vote {
                    address: Address::from_bytes(&address)?,
                    signature,
                })
            }
            let height = commit.proof.height;
            let latest_proof = Proof {
                height: commit.proof.height,
                round: commit.proof.round,
                proposal_hash: Hash::from_bytes(&commit.proof.block_hash)?,
                commits,
            };

            let ctx = Context::new();
            let status = self
                .engine
                .commit_block(ctx.clone(), proposal, latest_proof)
                .await?;

            // clear cache of last proposal.
            let mut proposal_origin = self.proposal_origin.write();
            proposal_origin.retain(|_, v| v.height > height);

            Ok(BftStatus {
                height:         status.height,
                interval:       Some(status.interval),
                authority_list: status
                    .verifier_list
                    .iter()
                    .map(|a| BftNode::set_address(a.as_bytes().to_vec()))
                    .collect(),
            })
        };

        block_on(fut)
    }

    /// A user-defined function for feeding the bft consensus.
    /// The new block provided will feed for bft consensus of giving [`height`]
    fn get_block(&self, _height: u64) -> Result<(Vec<u8>, Vec<u8>), Self::Error> {
        let fut = async move {
            let proposal = self.engine.build_proposal(Context::new()).await?;
            let proposal_hash = proposal.hash();
            let ser_proposal: SerProposal = proposal.into();

            let encoded = AsyncCodec::encode(ser_proposal).await?;
            Ok((encoded, proposal_hash.as_bytes().to_vec()))
        };

        block_on(fut)
    }

    /// A user-defined function for signing a [`hash`].
    fn sign(&self, hash: &[u8]) -> Result<Signature, Self::Error> {
        let hash = Hash::from_bytes(hash)?;
        let signature = self.engine.sign_with_hash(&hash)?;
        Ok(signature.as_bytes().to_vec())
    }

    /// A user-defined function for checking a [`signature`].
    fn check_sig(&self, signature: &[u8], hash: &[u8]) -> Result<BFTAddress, Self::Error> {
        let signature = C::Signature::from_bytes(signature)?;
        let hash = Hash::from_bytes(hash)?;
        let address = self.engine.verify_signature(&hash, &signature)?;

        Ok(address.as_bytes().to_vec())
    }

    /// A user-defined function for hashing a [`msg`].
    fn crypt_hash(&self, msg: &[u8]) -> Vec<u8> {
        Hash::digest(msg).as_bytes().to_vec()
    }
}
