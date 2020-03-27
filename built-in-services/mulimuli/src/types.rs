use serde::{Deserialize, Serialize};

use bytes::Bytes;
use molecule::prelude::Entity;

use protocol::fixed_codec::{FixedCodec, FixedCodecError};
use protocol::types::{Address, Hash, Hex};
use protocol::ProtocolResult;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GenesisPayload {
    pub assets: Vec<GenesisAsset>,
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
