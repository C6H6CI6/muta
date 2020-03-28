use serde::{Deserialize, Serialize};

use byteorder::{ByteOrder, LittleEndian};
use bytes::Bytes;
use molecule::prelude::Entity;

use protocol::fixed_codec::{FixedCodec, FixedCodecError};
use protocol::types::{Address, Hash, Hex};
use protocol::ProtocolResult;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct GenesisPayload {
    pub assets:      Vec<GenesisAsset>,
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

pub struct CKBTx {
    pub inner: ckb_types::packed::Transaction,
}

impl FixedCodec for CKBTx {
    fn encode_fixed(&self) -> ProtocolResult<bytes::Bytes> {
        Ok(self.inner.as_bytes())
    }

    fn decode_fixed(bytes: bytes::Bytes) -> ProtocolResult<Self> {
        let s = Self {
            inner: ckb_types::packed::Transaction::from_slice(&bytes).unwrap(),
        };
        Ok(s)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CKBCrossTxPayload {
    pub ckb_tx:  ckb_jsonrpc_types::Transaction,
    pub indices: Vec<u32>,
    pub lemmas:  Vec<Hash>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CKBTransferOutputData {
    pub address: Address,
    pub amount:  u64,
}

impl CKBTransferOutputData {
    pub fn from_slice(bytes: &[u8]) -> Self {
        let amount_bytes = &bytes[1..10];
        let address_bytes = &bytes[10..30];
        let amount = LittleEndian::read_u64(amount_bytes);
        let address = Address::from_bytes(Bytes::from(address_bytes.to_vec())).unwrap();
        println!("CKBTransferOutputData address {:?}", address);

        Self { amount, address }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct CKBDepositOutputData {
    pub address:     Address,
    pub bls_address: Hex,
    pub amount:      u64,
}

impl CKBDepositOutputData {
    pub fn from_slice(bytes: &[u8]) -> Self {
        let amount_bytes = &bytes[1..10];
        let address_bytes = &bytes[10..30];
        let bls_address_bytes = &bytes[30..bytes.len()];
        let amount = LittleEndian::read_u64(amount_bytes);
        let address = Address::from_bytes(Bytes::from(address_bytes.to_vec())).unwrap();
        let hex_str = hex::encode(bls_address_bytes);

        Self { amount, address, bls_address: Hex::from_string("0x".to_owned() + &hex_str).unwrap() }
    }
}
