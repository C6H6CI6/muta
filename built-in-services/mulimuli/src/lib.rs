#[cfg(test)]
mod tests;

mod types;

use byteorder::{ByteOrder, LittleEndian};
use bytes::{Bytes, BytesMut};
use derive_more::{Display, From};
use molecule::prelude::Entity;
use serde::{Deserialize, Serialize};

use binding_macro::{cycles, genesis, hook_after, read, service, write};
use protocol::traits::{ExecutorParams, ServiceSDK, StoreArray, StoreMap};
use protocol::types::{
    Address, Hash, Hex, Metadata, ServiceContext, ValidatorExtend, METADATA_KEY,
};
use protocol::{ProtocolError, ProtocolErrorKind, ProtocolResult};

use crate::types::{
    CKBCrossTxPayload, CKBDepositOutputData, CKBHeader, CKBTransferOutputData, CKBTx,
    CreateCommentResponse, CreatePostResponse, DeleteCommentPayload, DeletePostPayload,
    GenesisPayload, GetBalanceResponse, LatestCKBStatus, StarPayload, UpdateCKBPayload,
};

pub struct MulimuliService<SDK> {
    sdk:      SDK,
    assets:   Box<dyn StoreMap<Address, u64>>,
    posts:    Box<dyn StoreMap<Hash, Address>>,
    comments: Box<dyn StoreMap<Hash, Address>>,

    ckb_headers_map:    Box<dyn StoreMap<Bytes, CKBHeader>>,
    ckb_headers_vec:    Box<dyn StoreArray<CKBHeader>>,
    ckb_create_id_map:  Box<dyn StoreMap<Address, CKBTx>>,
    ckb_burn_id_map:    Box<dyn StoreMap<Address, CKBTx>>,
    ckb_deposit_id_map: Box<dyn StoreMap<Address, CKBTx>>,
    ckb_deposit_asset:  Box<dyn StoreMap<Address, u64>>,
}

#[service]
impl<SDK: ServiceSDK> MulimuliService<SDK> {
    pub fn new(mut sdk: SDK) -> ProtocolResult<Self> {
        let assets: Box<dyn StoreMap<Address, u64>> = sdk.alloc_or_recover_map("assets")?;
        let posts: Box<dyn StoreMap<Hash, Address>> = sdk.alloc_or_recover_map("posts")?;
        let comments: Box<dyn StoreMap<Hash, Address>> = sdk.alloc_or_recover_map("comments")?;
        let ckb_headers_map: Box<dyn StoreMap<Bytes, CKBHeader>> =
            sdk.alloc_or_recover_map("ckb_headers_map")?;
        let ckb_headers_vec: Box<dyn StoreArray<CKBHeader>> =
            sdk.alloc_or_recover_array("ckb_headers_vec")?;
        let ckb_create_id_map: Box<dyn StoreMap<Address, CKBTx>> =
            sdk.alloc_or_recover_map("ckb_create_id_map")?;
        let ckb_burn_id_map: Box<dyn StoreMap<Address, CKBTx>> =
            sdk.alloc_or_recover_map("ckb_burn_id_map")?;
        let ckb_deposit_id_map: Box<dyn StoreMap<Address, CKBTx>> =
            sdk.alloc_or_recover_map("ckb_deposit_id_map")?;
        let ckb_deposit_asset: Box<dyn StoreMap<Address, u64>> =
            sdk.alloc_or_recover_map("ckb_deposit_asset")?;

        Ok(Self {
            sdk,
            assets,
            posts,
            comments,
            ckb_headers_map,
            ckb_headers_vec,
            ckb_create_id_map,
            ckb_burn_id_map,
            ckb_deposit_id_map,
            ckb_deposit_asset,
        })
    }

    #[genesis]
    fn init_genesis(&mut self, payload: GenesisPayload) -> ProtocolResult<()> {
        let packed_header = ckb_types::packed::Header::from(payload.ckb_genesis);
        let block_hash = packed_header.calc_header_hash();
        let block_hash = Bytes::from(block_hash.as_bytes());
        self.ckb_headers_vec.push(CKBHeader {
            inner: packed_header.clone(),
        })?;
        self.ckb_headers_map.insert(block_hash, CKBHeader {
            inner: packed_header.clone(),
        })?;

        for asset in payload.assets.into_iter() {
            self.assets.insert(asset.address, asset.balance)?;
        }
        Ok(())
    }

    // fe
    #[read]
    fn get_balance(&self, ctx: ServiceContext) -> ProtocolResult<GetBalanceResponse> {
        let caller = ctx.get_caller();

        if !self.assets.contains(&caller)? {
            return Ok(GetBalanceResponse { balance: 0 });
        }

        let balance = self.assets.get(&caller)?;

        Ok(GetBalanceResponse { balance })
    }

    #[write]
    fn create_post(&mut self, ctx: ServiceContext) -> ProtocolResult<CreatePostResponse> {
        let caller = ctx.get_caller();

        let id = self.gen_id(ctx);
        if self.posts.contains(&id)? {
            return Err(ServiceError::PostExists { id }.into());
        }

        self.posts.insert(id.clone(), caller)?;
        Ok(CreatePostResponse { id })
    }

    #[read]
    fn get_deposit(&self, ctx: ServiceContext) -> ProtocolResult<GetBalanceResponse> {
        let caller = ctx.get_caller();

        if !self.ckb_deposit_asset.contains(&caller)? {
            return Ok(GetBalanceResponse { balance: 0 });
        }

        let balance = self.ckb_deposit_asset.get(&caller)?;

        Ok(GetBalanceResponse { balance })
    }

    #[write]
    fn delete_post(
        &mut self,
        _ctx: ServiceContext,
        payload: DeletePostPayload,
    ) -> ProtocolResult<()> {
        self.posts.remove(&payload.id)
    }

    #[write]
    fn deposit(
        &mut self,
        ctx: ServiceContext,
        payload: CKBCrossTxPayload,
    ) -> ProtocolResult<String> {
        let ckb_tx = ckb_types::packed::Transaction::from(payload.ckb_tx);
        let output_data = ckb_tx.raw().outputs_data().get(0).unwrap();
        let output_data = CKBDepositOutputData::from_slice(output_data.as_slice());

        if !self.ckb_deposit_asset.contains(&output_data.address)? {
            self.ckb_deposit_asset
                .insert(output_data.address.clone(), 0)?;
        }

        self.ckb_deposit_asset
            .insert(output_data.address.clone(), output_data.amount)?;

        self.ckb_deposit_id_map
            .insert(output_data.address.clone(), CKBTx { inner: ckb_tx.clone() })?;

        let json_str = self.sdk.read(&ctx, None, "metadata", "get_metadata", "")?;
        let mut metadata: Metadata =
            serde_json::from_str(&json_str).map_err(ServiceError::JsonParse)?;

        metadata.verifier_list.push(ValidatorExtend {
            bls_pub_key:    output_data.bls_address.clone(),
            address:        output_data.address.clone(),
            propose_weight: 1,
            vote_weight:    1,
        });

        let new_metadata = serde_json::to_string(&metadata).map_err(ServiceError::JsonParse)?;
        self.sdk
            .write(&ctx, None, "metadata", "write_metadata", &new_metadata)?;
        Ok(hex::encode(ckb_tx.raw().outputs_data().as_slice()))
    }

    #[write]
    fn refund(
        &mut self,
        ctx: ServiceContext,
        payload: CKBCrossTxPayload,
    ) -> ProtocolResult<String> {
        let ckb_tx = ckb_types::packed::Transaction::from(payload.ckb_tx);
        let output_data = ckb_tx.raw().outputs_data().get(0).unwrap();
        let output_data = CKBDepositOutputData::from_slice(output_data.as_slice());

        if !self.ckb_deposit_asset.contains(&output_data.address)? {
            return Err(ServiceError::NotFoundAddress {
                address: output_data.address,
            }
            .into());
        }

        self.ckb_deposit_asset.remove(&output_data.address)?;

        self.ckb_deposit_id_map.remove(&output_data.address)?;

        let json_str = self.sdk.read(&ctx, None, "metadata", "get_metadata", "")?;
        let mut metadata: Metadata =
            serde_json::from_str(&json_str).map_err(ServiceError::JsonParse)?;

        let list: Vec<ValidatorExtend> = metadata
            .verifier_list
            .into_iter()
            .filter(|v| v.address != output_data.address)
            .collect();
        metadata.verifier_list = list;

        let new_metadata = serde_json::to_string(&metadata).map_err(ServiceError::JsonParse)?;
        self.sdk
            .write(&ctx, None, "metadata", "write_metadata", &new_metadata)?;

        Ok(hex::encode(ckb_tx.raw().outputs_data().as_slice()))
    }

    #[write]
    fn create_comment(&mut self, ctx: ServiceContext) -> ProtocolResult<CreateCommentResponse> {
        let caller = ctx.get_caller();

        let id = self.gen_id(ctx);
        if self.comments.contains(&id)? {
            return Err(ServiceError::CommentExists { id }.into());
        }

        self.comments.insert(id.clone(), caller)?;
        Ok(CreateCommentResponse { id })
    }

    #[write]
    fn delete_comment(
        &mut self,
        _ctx: ServiceContext,
        payload: DeleteCommentPayload,
    ) -> ProtocolResult<()> {
        self.posts.remove(&payload.id)
    }

    #[write]
    fn star(&mut self, ctx: ServiceContext, payload: StarPayload) -> ProtocolResult<()> {
        if !self.assets.contains(&ctx.get_caller())? {
            return Err(ServiceError::NotFoundAddress {
                address: ctx.get_caller().clone(),
            }
            .into());
        }

        let amount = self.assets.get(&ctx.get_caller())?;

        let (v, overflow) = amount.overflowing_sub(payload.balance);
        if overflow {
            return Err(ServiceError::U64Overflow.into());
        }

        self.assets.insert(ctx.get_caller().clone(), v)?;

        let address = self.posts.get(&payload.post_id)?;

        if !self.assets.contains(&address)? {
            self.assets.insert(address.clone(), 0)?;
        }

        let balance = self.assets.get(&address)?;
        self.assets.insert(address, balance + payload.balance)?;
        Ok(())
    }

    fn gen_id(&self, ctx: ServiceContext) -> Hash {
        let caller = ctx.get_caller().as_bytes();
        let nonce = ctx.get_nonce().unwrap().as_bytes();
        let height = ctx.get_current_height();
        let mut height_buf = [0u8; std::mem::size_of::<u64>()];
        LittleEndian::write_u64(&mut height_buf, height);

        let mut bm = BytesMut::new();
        bm.extend_from_slice(&caller);
        bm.extend_from_slice(&nonce);
        bm.extend_from_slice(&height_buf);

        Hash::digest(bm.freeze())
    }

    #[read]
    fn get_ckb_status(&self, ctx: ServiceContext) -> ProtocolResult<LatestCKBStatus> {
        let len = self.ckb_headers_vec.len()?;
        if len == 0 {
            Ok(LatestCKBStatus { height: None })
        } else {
            let header = self.ckb_headers_vec.get(len - 1)?;

            Ok(LatestCKBStatus {
                height: Some(get_height(&header.inner)),
            })
        }
    }

    #[write]
    fn create_asset(
        &mut self,
        ctx: ServiceContext,
        payload: CKBCrossTxPayload,
    ) -> ProtocolResult<String> {
        let ckb_tx = ckb_types::packed::Transaction::from(payload.ckb_tx);
        let output_data = ckb_tx.raw().outputs_data().get(0).unwrap();
        let create_data = CKBTransferOutputData::from_slice(output_data.as_slice());

        if !self.assets.contains(&create_data.address)? {
            self.assets.insert(create_data.address.clone(), 0)?;
        }

        let balance = self.assets.get(&create_data.address)?;
        self.assets
            .insert(create_data.address.clone(), balance + create_data.amount)?;
        self.ckb_create_id_map
            .insert(create_data.address.clone(), CKBTx { inner: ckb_tx })?;

        Ok(hex::encode(output_data.as_slice()))
    }

    #[write]
    fn burn_asset(
        &mut self,
        ctx: ServiceContext,
        payload: CKBCrossTxPayload,
    ) -> ProtocolResult<String> {
        let ckb_tx = ckb_types::packed::Transaction::from(payload.ckb_tx);
        let output_data = ckb_tx.raw().outputs_data().get(0).unwrap();
        let transfer_data = CKBTransferOutputData::from_slice(output_data.as_slice());

        let balance = self.assets.get(&transfer_data.address)?;
        self.assets.insert(
            transfer_data.address.clone(),
            balance - transfer_data.amount,
        )?;
        self.ckb_burn_id_map
            .insert(transfer_data.address.clone(), CKBTx { inner: ckb_tx })?;

        Ok(hex::encode(output_data.as_slice()))
    }

    // ckb
    #[write]
    fn update_ckb(&mut self, ctx: ServiceContext, payload: UpdateCKBPayload) -> ProtocolResult<()> {
        if payload.headers.is_empty() {
            return Ok(());
        }

        let len = self.ckb_headers_vec.len()?;
        let mut latest_header = self.ckb_headers_vec.get(len - 1)?;

        for header in payload.headers.into_iter() {
            let packed_header = ckb_types::packed::Header::from(header);
            let prev_hash = latest_header.inner.calc_header_hash();

            if get_height(&latest_header.inner) + 1 != get_height(&packed_header) {
                return Err(ServiceError::InvalidHeight {
                    height: get_height(&packed_header),
                }
                .into());
            }

            if prev_hash != packed_header.raw().parent_hash() {
                return Err(ServiceError::InvalidHeight {
                    height: get_height(&packed_header),
                }
                .into());
            }

            let block_hash = packed_header.calc_header_hash();
            let block_hash = Bytes::from(block_hash.as_bytes());
            self.ckb_headers_vec.push(CKBHeader {
                inner: packed_header.clone(),
            })?;
            self.ckb_headers_map.insert(block_hash, CKBHeader {
                inner: packed_header.clone(),
            })?;
            latest_header = CKBHeader {
                inner: packed_header,
            };
        }
        Ok(())
    }
}

fn get_height(header: &ckb_types::packed::Header) -> u64 {
    let slice = header.raw().number();
    LittleEndian::read_u64(slice.as_slice())
}

#[derive(Debug, Display, From)]
pub enum ServiceError {
    #[display(fmt = "Parsing payload to json failed {:?}", _0)]
    JsonParse(serde_json::Error),

    #[display(fmt = "Post {:?} already exists", id)]
    PostExists {
        id: Hash,
    },

    #[display(fmt = "Comment {:?} already exists", id)]
    CommentExists {
        id: Hash,
    },

    #[display(fmt = "Not found address, address {:?}", address)]
    NotFoundAddress {
        address: Address,
    },

    #[display(fmt = "id {:?} already exists", id)]
    IDExists {
        id: Address,
    },

    #[display(fmt = "Invalid height {:?}", height)]
    InvalidHeight {
        height: u64,
    },

    #[display(fmt = "Not found asset, expect {:?} real {:?}", expect, real)]
    LackOfBalance {
        expect: u64,
        real:   u64,
    },

    U64Overflow,

    RecipientIsSender,

    ApproveToYourself,
}

impl std::error::Error for ServiceError {}

impl From<ServiceError> for ProtocolError {
    fn from(err: ServiceError) -> ProtocolError {
        ProtocolError::new(ProtocolErrorKind::Service, Box::new(err))
    }
}