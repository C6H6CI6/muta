use std::error::Error;
use std::fmt;

use core_context::Context;

use crate::BoxFuture;

/// Specify the category of data stored, and users can store the data in a
/// decentralized manner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataCategory {
    // Block
    Block,
    // Already of "SignedTransaction" in the block.
    Transaction,
    // Already of "Receipt" in the block.
    Receipt,
    // State of the world
    State,
    // "SignedTransaction" in the transaction pool
    TransactionPool,
    // Transaction position in block
    TransactionPosition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DatabaseError {
    NotFound,
    InvalidData,
    Internal(String),
}

impl Error for DatabaseError {}
impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let printable = match *self {
            DatabaseError::NotFound => "not found".to_owned(),
            DatabaseError::InvalidData => "invalid data".to_owned(),
            DatabaseError::Internal(ref err) => format!("internal error: {:?}", err),
        };
        write!(f, "{}", printable)
    }
}

pub type FutDBResult<T> = BoxFuture<'static, Result<T, DatabaseError>>;

pub trait Database: Send + Sync {
    fn get(&self, ctx: Context, c: DataCategory, key: &[u8]) -> FutDBResult<Option<Vec<u8>>>;

    fn get_batch(
        &self,
        ctx: Context,
        c: DataCategory,
        keys: &[Vec<u8>],
    ) -> FutDBResult<Vec<Option<Vec<u8>>>>;

    fn insert(
        &self,
        ctx: Context,
        c: DataCategory,
        key: Vec<u8>,
        value: Vec<u8>,
    ) -> FutDBResult<()>;

    fn insert_batch(
        &self,
        ctx: Context,
        c: DataCategory,
        keys: Vec<Vec<u8>>,
        values: Vec<Vec<u8>>,
    ) -> FutDBResult<()>;

    fn contains(&self, ctx: Context, c: DataCategory, key: &[u8]) -> FutDBResult<bool>;

    fn remove(&self, ctx: Context, c: DataCategory, key: &[u8]) -> FutDBResult<()>;

    fn remove_batch(&self, ctx: Context, c: DataCategory, keys: &[Vec<u8>]) -> FutDBResult<()>;
}
