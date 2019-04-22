use std::error;
use std::fmt;
use std::io;

use core_crypto::CryptoError;
use core_runtime::executor::ExecutorError;
use core_runtime::TransactionPoolError;
use core_serialization::CodecError;
use core_storage::StorageError;

#[derive(Debug)]
pub enum RpcError {
    Str(String),
    CodecError(CodecError),
    CryptoError(CryptoError),
    ExecutorError(ExecutorError),
    StorageError(StorageError),
    TransactionPoolError(TransactionPoolError),
    IO(io::Error),
}

impl error::Error for RpcError {}
impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RpcError::Str(e) => return write!(f, "{}", e),
            RpcError::CodecError(e) => return write!(f, "{}", e),
            RpcError::CryptoError(e) => return write!(f, "{}", e),
            RpcError::ExecutorError(e) => return write!(f, "{}", e),
            RpcError::StorageError(e) => return write!(f, "{}", e),
            RpcError::TransactionPoolError(e) => return write!(f, "{}", e),
            RpcError::IO(e) => return write!(f, "{}", e),
        };
    }
}

impl From<CodecError> for RpcError {
    fn from(error: CodecError) -> Self {
        RpcError::CodecError(error)
    }
}

impl From<StorageError> for RpcError {
    fn from(error: StorageError) -> Self {
        RpcError::StorageError(error)
    }
}

impl From<TransactionPoolError> for RpcError {
    fn from(error: TransactionPoolError) -> Self {
        RpcError::TransactionPoolError(error)
    }
}

impl From<io::Error> for RpcError {
    fn from(error: io::Error) -> Self {
        RpcError::IO(error)
    }
}

impl From<CryptoError> for RpcError {
    fn from(error: CryptoError) -> Self {
        RpcError::CryptoError(error)
    }
}

impl From<ExecutorError> for RpcError {
    fn from(error: ExecutorError) -> Self {
        RpcError::ExecutorError(error)
    }
}
