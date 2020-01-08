extern crate test;

macro_rules! exec {
    ($func: expr) => {
        futures::executor::block_on(async { $func.await.unwrap() })
    };
}

mod adapter;
mod storage;

use rand::random;

use protocol::types::{
    Address, Epoch, EpochHeader, Hash, Proof, RawTransaction, Receipt, ReceiptResponse,
    SignedTransaction, TransactionRequest,
};
use protocol::Bytes;

fn mock_signed_tx(tx_hash: Hash) -> SignedTransaction {
    let nonce = Hash::digest(Bytes::from("XXXX"));

    let request = TransactionRequest {
        service_name: "test".to_owned(),
        method:       "test".to_owned(),
        payload:      "test".to_owned(),
    };

    let raw = RawTransaction {
        chain_id: nonce.clone(),
        nonce,
        timeout: 10,
        cycles_limit: 10,
        cycles_price: 1,
        request,
    };

    SignedTransaction {
        raw,
        tx_hash,
        pubkey: Default::default(),
        signature: Default::default(),
    }
}

fn mock_receipt(tx_hash: Hash) -> Receipt {
    let nonce = Hash::digest(Bytes::from("XXXX"));

    let response = ReceiptResponse {
        service_name: "test".to_owned(),
        method:       "test".to_owned(),
        ret:          "test".to_owned(),
        is_error:     false,
    };
    Receipt {
        state_root: nonce,
        epoch_id: 10,
        tx_hash,
        cycles_used: 10,
        events: vec![],
        response,
    }
}

fn mock_epoch(epoch_id: u64, epoch_hash: Hash) -> Epoch {
    let nonce = Hash::digest(Bytes::from("XXXX"));
    let addr_str = "CAB8EEA4799C21379C20EF5BAA2CC8AF1BEC475B";
    let header = EpochHeader {
        chain_id: nonce.clone(),
        epoch_id,
        exec_epoch_id: epoch_id - 1,
        pre_hash: nonce.clone(),
        timestamp: 1000,
        logs_bloom: Default::default(),
        order_root: nonce.clone(),
        confirm_root: Vec::new(),
        state_root: nonce,
        receipt_root: Vec::new(),
        cycles_used: vec![999_999],
        proposer: Address::from_hex(addr_str).unwrap(),
        proof: mock_proof(epoch_hash),
        validator_version: 1,
        validators: Vec::new(),
    };

    Epoch {
        header,
        ordered_tx_hashes: Vec::new(),
    }
}

fn mock_proof(epoch_hash: Hash) -> Proof {
    Proof {
        epoch_id: 0,
        round: 0,
        epoch_hash,
        signature: Default::default(),
        bitmap: Default::default(),
    }
}

fn get_random_bytes(len: usize) -> Bytes {
    let vec: Vec<u8> = (0..len).map(|_| random::<u8>()).collect();
    Bytes::from(vec)
}
