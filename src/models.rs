use diesel::prelude::*;
use crate::schema::*;
use crate::block::BlockHeader;

#[derive(Queryable)]
#[derive(Insertable)]
#[table_name="blocks"]
pub struct Block {
    pub hash: Vec<u8>,
    pub height: i32,
    pub version: i32,
    pub prev_block: Vec<u8>,
    pub merkle_root: Vec<u8>,
    pub timestamp: i64,
    pub bits: i64,
    pub nonce: i64,
}

#[derive(Queryable)]
pub struct Token {
    pub id:                   i32,    // SERIAL PRIMARY KEY,
    pub hash:                 Vec<u8>, // BYTEA    NOT NULL,
    pub decimals:             i32,    // INT      NOT NULL,
    pub timestamp:            i64,    // BIGINT   NOT NULL,
    pub version_type:         i16,    // SMALLINT NOT NULL,
    pub document_uri:         Option<String>, // VARCHAR(200),
    pub symbol:               Option<String>, // VARCHAR(200),
    pub name:                 Option<String>, // VARCHAR(200),
    pub document_hash:        Option<String>, // VARCHAR(200),
    pub initial_supply:       i64,    // BIGINT NOT NULL,
    pub current_supply:       i64,    // BIGINT NOT NULL,
    pub block_created_height: i32,    // INT NOT NULL
}

#[derive(Insertable)]
#[table_name="token"]
pub struct NewToken {
    pub hash:                 Vec<u8>, // BYTEA    NOT NULL,
    pub decimals:             i32,    // INT      NOT NULL,
    pub timestamp:            i64,    // BIGINT   NOT NULL,
    pub version_type:         i16,    // SMALLINT NOT NULL,
    pub document_uri:         Option<String>, // VARCHAR(200),
    pub symbol:               Option<String>, // VARCHAR(200),
    pub name:                 Option<String>, // VARCHAR(200),
    pub document_hash:        Option<String>, // VARCHAR(200),
    pub initial_supply:       i64,    // BIGINT NOT NULL,
    pub current_supply:       i64,    // BIGINT NOT NULL,
    pub block_created_height: i32,    // INT NOT NULL
}

#[derive(Queryable)]
pub struct Tx {
    pub id:        i64, // BIGSERIAL PRIMARY KEY,
    pub hash:      Vec<u8>, // BYTEA NOT NULL,
    pub height:    Option<i32>, // INT NOT NULL,
    pub timestamp: i64, // BIGINT NOT NULL,
    pub tx_type:   i32, // INT NOT NULL
}

#[derive(Insertable)]
#[table_name="tx"]
pub struct NewTx {
    pub hash:      Vec<u8>, // BYTEA NOT NULL,
    pub height:    Option<i32>, // INT NOT NULL,
    pub timestamp: i64, // BIGINT NOT NULL,
    pub tx_type:   i32, // INT NOT NULL
}

#[derive(Queryable)]
#[derive(Insertable)]
#[table_name="slp_tx"]
pub struct SlpTx {
    pub tx:       i64, // BIGINT PRIMARY KEY REFERENCES tx ("id") ON DELETE CASCADE,
    pub token:    i32, // INT REFERENCES token ("id") ON DELETE RESTRICT,
    pub version:  i32, // INT NOT NULL,
    pub slp_type: String, // VARCHAR(14) NOT NULL
}

#[derive(Queryable)]
#[derive(Insertable)]
#[table_name="tx_output"]
pub struct TxOutput {
    pub tx:               i64, // BIGINT REFERENCES tx (id) ON DELETE CASCADE,
    pub idx:              i32, // INT NOT NULL,
    pub value_satoshis:   i64, // BIGINT NOT NULL,
    pub value_token_base: i64, // BIGINT NOT NULL,
    pub address:          Option<Vec<u8>>, // BYTEA,
    pub output_type:      i32, // INT NOT NULL,
}

#[derive(Queryable)]
#[derive(Insertable)]
#[table_name="tx_input"]
pub struct TxInput {
    pub tx:         i64, // BIGINT REFERENCES tx (id) ON DELETE CASCADE,
    pub idx:        i32, // INT NOT NULL,
    pub output_tx:  Vec<u8>, // BIGINT,  -- can be null
    pub output_idx: i32, // INT,
    pub address:    Option<Vec<u8>>, // BYTEA
}

#[derive(Queryable)]
pub struct TradeOffer {
    pub id:                          i64, // SERIAL PRIMARY KEY,
    pub tx:                          i64, // BIGINT REFERENCES tx (id) ON DELETE CASCADE,
    pub output_idx:                  Option<i32>, // INT NOT NULL,
    pub input_tx:                    Vec<u8>, // BYTEA NOT NULL,
    pub input_idx:                   i32, // INT NOT NULL,
    pub approx_price_per_token:      f64, // DOUBLE PRECISION NOT NULL,
    pub price_per_token_nominator:   i64, // BIGINT NOT NULL,
    pub price_per_token_denominator: i64, // BIGINT NOT NULL,
    pub script_price:                i64, // BIGINT NOT NULL,
    pub sell_amount_token_base:      i64, // INT NOT NULL,
    pub receiving_address:           Vec<u8>, // BYTEA NOT NULL,
    pub spent:                       bool, // BOOL NOT NULL
}

#[derive(Insertable)]
#[table_name="trade_offer"]
pub struct NewTradeOffer {
    pub tx:                     i64, // BIGINT REFERENCES tx (id) ON DELETE CASCADE,
    pub output_idx:             Option<i32>, // INT NOT NULL,
    pub input_tx:               Vec<u8>, // BYTEA NOT NULL,
    pub input_idx:              i32, // INT NOT NULL,
    pub approx_price_per_token: f64, // DOUBLE PRECISION NOT NULL,
    pub price_per_token_numer:  i64, // BIGINT NOT NULL,
    pub price_per_token_denom:  i64, // BIGINT NOT NULL,
    pub script_price:           i64, // BIGINT NOT NULL,
    pub sell_amount_token_base: i64, // INT NOT NULL,
    pub receiving_address:      Vec<u8>, // BYTEA NOT NULL
}

#[derive(Queryable)]
pub struct UpdateHistory {
    pub id:              i64, // BIGSERIAL PRIMARY KEY,
    pub last_height:     i32, // INT NOT NULL,
    pub last_tx_hash:    Option<Vec<u8>>, // BYTEA NOT NULL,
    pub last_tx_hash_be: Option<Vec<u8>>, // BYTEA NOT NULL,
    pub subject_type:    i32, // INT NOT NULL,
    pub subject_hash:    Option<Vec<u8>>, // INT NOT NULL,
    pub timestamp:       chrono::DateTime<chrono::Utc>, // TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    pub completed:       bool, // BOOL NOT NULL
}

#[derive(Insertable)]
#[table_name="update_history"]
pub struct NewUpdateHistory {
    pub last_height:     i32, // INT NOT NULL,
    pub last_tx_hash:    Option<Vec<u8>>, // BYTEA NOT NULL,
    pub last_tx_hash_be: Option<Vec<u8>>, // BYTEA NOT NULL,
    pub subject_type:    i32, // INT NOT NULL,
    pub subject_hash:    Option<Vec<u8>>, // BYTEA NOT NULL,
    pub completed:       bool, // BOOL NOT NULL
}

impl Block {
    pub fn from_block_header(header: &BlockHeader, height: i32) -> Block {
        Block {
            hash: header.hash().to_vec(),
            height,
            version: header.version,
            prev_block: header.prev_block.to_vec(),
            merkle_root: header.merkle_root.to_vec(),
            timestamp: header.timestamp as i64,
            bits: header.bits as i64,
            nonce: header.nonce as i64,
        }
    }

    pub fn to_block_header(&self) -> BlockHeader {
        BlockHeader {
            version: self.version,
            prev_block: {
                let mut prev_block = [0; 32];
                prev_block.copy_from_slice(&self.prev_block);
                prev_block
            },
            merkle_root: {
                let mut merkle_root = [0; 32];
                merkle_root.copy_from_slice(&self.merkle_root);
                merkle_root
            },
            timestamp: self.timestamp as u32,
            bits: self.bits as u32,
            nonce: self.nonce as u32,
        }
    }
}
