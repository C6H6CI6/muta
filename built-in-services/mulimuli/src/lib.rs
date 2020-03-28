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
use protocol::types::{Address, Hash, Hex, Metadata, ServiceContext, METADATA_KEY};
use protocol::{ProtocolError, ProtocolErrorKind, ProtocolResult};

use crate::types::{
    CKBHeader, CKBTransferOutputData, CKBTransferPayload, CKBTx, CreateCommentResponse,
    CreatePostResponse, DeleteCommentPayload, DeletePostPayload, GenesisPayload,
    GetBalanceResponse, LatestCKBStatus, StarPayload, UpdateCKBPayload,
};

pub struct MulimuliService<SDK> {
    sdk:      SDK,
    assets:   Box<dyn StoreMap<Address, u64>>,
    posts:    Box<dyn StoreMap<Hash, Address>>,
    comments: Box<dyn StoreMap<Hash, Address>>,

    ckb_headers_map:   Box<dyn StoreMap<Bytes, CKBHeader>>,
    ckb_headers_vec:   Box<dyn StoreArray<CKBHeader>>,
    ckb_create_id_map: Box<dyn StoreMap<u64, CKBTx>>,
    ckb_burn_id_map:   Box<dyn StoreMap<u64, CKBTx>>,
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
        let ckb_create_id_map: Box<dyn StoreMap<u64, CKBTx>> =
            sdk.alloc_or_recover_map("ckb_create_id_map")?;
        let ckb_burn_id_map: Box<dyn StoreMap<u64, CKBTx>> =
            sdk.alloc_or_recover_map("ckb_burn_id_map")?;

        Ok(Self {
            sdk,
            assets,
            posts,
            comments,
            ckb_headers_map,
            ckb_headers_vec,
            ckb_create_id_map,
            ckb_burn_id_map,
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

    #[write]
    fn delete_post(
        &mut self,
        _ctx: ServiceContext,
        payload: DeletePostPayload,
    ) -> ProtocolResult<()> {
        self.posts.remove(&payload.id)
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
        payload: CKBTransferPayload,
    ) -> ProtocolResult<CKBTransferOutputData> {
        let ckb_tx = ckb_types::packed::Transaction::from(payload.ckb_tx);
        let output_data = ckb_tx.raw().outputs_data();
        let create_data: CKBTransferOutputData =
            serde_json::from_slice(output_data.as_slice()).map_err(ServiceError::JsonParse)?;

        if self.ckb_create_id_map.contains(&create_data.id)? {
            return Err(ServiceError::IDExists { id: create_data.id }.into());
        }

        if !self.assets.contains(&create_data.address)? {
            self.assets.insert(create_data.address.clone(), 0)?;
        }

        let balance = self.assets.get(&create_data.address)?;
        self.assets
            .insert(create_data.address.clone(), balance + create_data.amount)?;
        self.ckb_create_id_map
            .insert(create_data.id, CKBTx { inner: ckb_tx })?;

        Ok(create_data)
    }

    #[write]
    fn burn_asset(
        &mut self,
        ctx: ServiceContext,
        payload: CKBTransferPayload,
    ) -> ProtocolResult<CKBTransferOutputData> {
        let ckb_tx = ckb_types::packed::Transaction::from(payload.ckb_tx);
        let output_data = ckb_tx.raw().outputs_data();
        let transfer_data: CKBTransferOutputData =
            serde_json::from_slice(output_data.as_slice()).map_err(ServiceError::JsonParse)?;

        if self.ckb_burn_id_map.contains(&transfer_data.id)? {
            return Err(ServiceError::IDExists {
                id: transfer_data.id,
            }
            .into());
        }

        let balance = self.assets.get(&transfer_data.address)?;
        self.assets.insert(
            transfer_data.address.clone(),
            balance - transfer_data.amount,
        )?;
        self.ckb_burn_id_map
            .insert(transfer_data.id, CKBTx { inner: ckb_tx })?;

        Ok(transfer_data)
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
        id: u64,
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
