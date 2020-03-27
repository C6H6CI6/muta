use serde::{Deserialize, Serialize};

use protocol::fixed_codec::{FixedCodec, FixedCodecError};
use protocol::types::{Address, Hash};

pub struct UpdateCKBBlockPayload {}

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

// impl rlp::Decodable for GetBalanceResponse {
//     fn decode(rlp: &rlp::Rlp) -> Result<Self, rlp::DecoderError> {
//         Ok(Self {
//             id: rlp.at(0)?.as_val()?,
//         })
//     }
// }
//
// impl rlp::Encodable for GetBalanceResponse {
//     fn rlp_append(&self, s: &mut rlp::RlpStream) {
//         s.begin_list(1).append(&self.balance);
//     }
// }
