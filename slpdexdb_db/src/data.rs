use slpdexdb_base::SLPAmount;

pub struct Utxo {
    pub tx_hash: [u8; 32],
    pub vout: i32,
    pub value_satoshis: u64,
    pub value_token: SLPAmount,
    pub token_hash: Option<[u8; 32]>,
}

#[derive(Clone, Debug)]
pub struct TxDelta {
    pub tx_hash: [u8; 32],
    pub delta_satoshis: i64,
    pub delta_token: SLPAmount,
    pub token_hash: Option<[u8; 32]>,
    pub timestamp: i64,
}
