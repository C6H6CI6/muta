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
    CKBHeader, CreateCommentResponse, CreatePostResponse, DeleteCommentPayload, DeletePostPayload,
    GenesisPayload, GetBalanceResponse, StarPayload, UpdateCKBPayload,
};

pub struct MulimuliService<SDK> {
    sdk:      SDK,
    assets:   Box<dyn StoreMap<Address, u64>>,
    posts:    Box<dyn StoreMap<Hash, Address>>,
    comments: Box<dyn StoreMap<Hash, Address>>,

    star_map:        Box<dyn StoreMap<Hash, u64>>,
    ckb_headers_map: Box<dyn StoreMap<Bytes, CKBHeader>>,
    ckb_headers_vec: Box<dyn StoreArray<CKBHeader>>,
}

#[service]
impl<SDK: ServiceSDK> MulimuliService<SDK> {
    pub fn new(mut sdk: SDK) -> ProtocolResult<Self> {
        let assets: Box<dyn StoreMap<Address, u64>> = sdk.alloc_or_recover_map("assets")?;
        let posts: Box<dyn StoreMap<Hash, Address>> = sdk.alloc_or_recover_map("posts")?;
        let comments: Box<dyn StoreMap<Hash, Address>> = sdk.alloc_or_recover_map("comments")?;
        let star_map: Box<dyn StoreMap<Hash, u64>> = sdk.alloc_or_recover_map("star_map")?;
        let ckb_headers_map: Box<dyn StoreMap<Bytes, CKBHeader>> =
            sdk.alloc_or_recover_map("ckb_headers_map")?;
        let ckb_headers_vec: Box<dyn StoreArray<CKBHeader>> =
            sdk.alloc_or_recover_array("ckb_headers_vec")?;

        Ok(Self {
            sdk,
            assets,
            posts,
            comments,
            star_map,
            ckb_headers_map,
            ckb_headers_vec,
        })
    }

    #[genesis]
    fn init_genesis(&mut self, payload: GenesisPayload) -> ProtocolResult<()> {
        for asset in payload.assets.into_iter() {
            self.assets.insert(asset.address, asset.balance)?;
        }
        Ok(())
    }

    #[hook_after]
    fn blocl_book_after(&mut self, _params: &ExecutorParams) -> ProtocolResult<()> {
        for (post_id, balance) in self.star_map.iter() {
            let address = self.posts.get(&post_id)?;

            let amount = self.assets.get(&address)?;
            // let author_dividend = balance / 2;
            // let leftover = balance - author_dividend;
            // self.assets.insert(address, amount + author_dividend)?;
            self.assets.insert(address, amount + balance)?;
        }

        Ok(())
    }

    #[read]
    fn create_asset(&self, _ctx: ServiceContext) -> ProtocolResult<Metadata> {
        let metadata: Metadata = self
            .sdk
            .get_value(&METADATA_KEY.to_owned())?
            .expect("Metadata should always be in the genesis block");
        Ok(metadata)
    }

    // fe
    #[read]
    fn get_balance(&self, ctx: ServiceContext) -> ProtocolResult<GetBalanceResponse> {
        let caller = ctx.get_caller();

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

        if !self.star_map.contains(&payload.post_id)? {
            self.star_map.insert(payload.post_id.clone(), 0)?;
        }
        let count = self.star_map.get(&payload.post_id)?;
        self.star_map
            .insert(payload.post_id, count + payload.balance)?;
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

    // ckb
    #[write]
    fn update_ckb(&mut self, ctx: ServiceContext, payload: UpdateCKBPayload) -> ProtocolResult<()> {
        if payload.headers.is_empty() {
            return Ok(());
        }

        let len = self.ckb_headers_vec.len()?;
        let mut latest_header = if len == 0 {
            let packed_header =
                ckb_types::packed::Header::from(payload.headers.first().unwrap().clone());
            let block_hash = packed_header.calc_header_hash();
            let block_hash = Bytes::from(block_hash.as_bytes());
            self.ckb_headers_vec.push(CKBHeader {
                inner: packed_header.clone(),
            })?;
            self.ckb_headers_map.insert(block_hash, CKBHeader {
                inner: packed_header.clone(),
            })?;
            CKBHeader {
                inner: packed_header,
            }
        } else {
            self.ckb_headers_vec.get(len - 1)?
        };

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
