use serde::{Deserialize, Serialize};

use bytes::Bytes;
use molecule::prelude::Entity;

use protocol::fixed_codec::{FixedCodec, FixedCodecError};
use protocol::types::{Address, Hash, Hex};
use protocol::ProtocolResult;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GenesisPayload {
    pub assets: Vec<GenesisAsset>,
    pub ckb_genesis: ckb_jsonrpc_types::Header,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GenesisAsset {
    pub address: Address,
    pub balance: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GetBalanceResponse {
    pub balance: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DeletePostPayload {
    pub id: Hash,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct DeleteCommentPayload {
    pub id: Hash,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct StarPayload {
    pub post_id: Hash,
    pub balance: u64,
}


#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CreatePostResponse {
    pub id: Hash,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CreateCommentResponse {
    pub id: Hash,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct UpdateCKBPayload {
    pub headers: Vec<ckb_jsonrpc_types::Header>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct LatestCKBStatus {
    pub height: Option<u64>,
}


pub struct CKBHeader {
    pub inner: ckb_types::packed::Header,
}

impl FixedCodec for CKBHeader {
    fn encode_fixed(&self) -> ProtocolResult<bytes::Bytes> {
        Ok(self.inner.as_bytes())
    }

    fn decode_fixed(bytes: bytes::Bytes) -> ProtocolResult<Self> {
        let s = Self {
            inner: ckb_types::packed::Header::from_slice(&bytes).unwrap(),
        };
        Ok(s)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CreateAssetPayload {
    pub ckb_tx: ckb_jsonrpc_types::Transaction,
    pub merkle_path: Vec<Hex>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CKBTransferOutputData {
    pub id: u64,
    pub address: Address,
    pub amount: u64,
}

