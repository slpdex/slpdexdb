use crate::token_source::token_result::TokenEntry;
use slpdexdb_base::SLPAmount;
use slpdexdb_base::{Result, ErrorKind, Error, TokenError};

#[derive(Clone, Debug)]
pub struct Token {
    pub hash:                 [u8; 32],
    pub decimals:             i32,
    pub timestamp:            i64,
    pub version_type:         i16,
    pub document_uri:         Option<String>,
    pub symbol:               Option<String>,
    pub name:                 Option<String>,
    pub document_hash:        Option<String>,
    pub initial_supply:       SLPAmount,
    pub current_supply:       SLPAmount,
    pub block_created_height: i32,
}

impl Token {
    pub fn str_or_empty(string: String) -> Option<String> {
        if string.is_empty() { None } else { Some(string) }
    }

    pub fn from_entry(token_entry: TokenEntry) -> Result<Self> {
        let not_mined_yet_err = || -> Error {
            ErrorKind::TokenError(
                TokenError::TokenNotMinedYet(token_entry.token_details.token_id_hex.clone())
            ).into()
        };
        Ok(Token {
            hash: {
                let mut hash = [0; 32];
                hash.copy_from_slice(&hex::decode(&token_entry.token_details.token_id_hex)?);
                hash
            },
            decimals: token_entry.token_details.decimals,
            timestamp: token_entry.token_details.timestamp_unix.ok_or_else(not_mined_yet_err)?,
            block_created_height: token_entry.token_stats.block_created.ok_or_else(not_mined_yet_err)?,
            version_type: token_entry.token_details.version_type,
            document_uri: Self::str_or_empty(token_entry.token_details.document_uri),
            symbol: Self::str_or_empty(token_entry.token_details.symbol),
            name: Self::str_or_empty(token_entry.token_details.name),
            document_hash: token_entry.token_details.document_sha256_hex
                .and_then(|hash| Self::str_or_empty(hash)),
            initial_supply: SLPAmount::from_str_decimals(
                &token_entry.token_details.genesis_or_mint_quantity,
                token_entry.token_details.decimals as u32,
            )?,
            current_supply: SLPAmount::from_str_decimals(
                &token_entry.token_stats.qty_token_circulating_supply,
                token_entry.token_details.decimals as u32,
            )?,
        })
    }
}
