use crate::token_source::token_result::TokenEntry;

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
    pub initial_supply:       u64,
    pub current_supply:       u64,
    pub block_created_height: i32,
}

impl Token {
    pub fn str_or_empty(string: String) -> Option<String> {
        if string.is_empty() { None } else { Some(string) }
    }

    pub fn parse_amount(decimals: u32, amount_str: &str) -> Option<u64> {
        let prec = 128;
        let amount = rug::Float::with_val(prec, rug::Float::parse(amount_str).ok()?);
        let factor = rug::Float::with_val(prec, rug::Float::u_pow_u(10, decimals));
        Some((amount * &factor).to_integer()?.to_u128()? as u64)
    }

    pub fn from_entry(token_entry: TokenEntry) -> Option<Self> {
        Some(Token {
            hash: {
                let mut hash = [0; 32];
                hash.copy_from_slice(&hex::decode(&token_entry.token_details.token_id_hex).ok()?);
                hash
            },
            decimals: token_entry.token_details.decimals,
            timestamp: token_entry.token_details.timestamp_unix?,
            version_type: token_entry.token_details.version_type,
            document_uri: Self::str_or_empty(token_entry.token_details.document_uri),
            symbol: Self::str_or_empty(token_entry.token_details.symbol),
            name: Self::str_or_empty(token_entry.token_details.name),
            document_hash: token_entry.token_details.document_sha256_hex
                .and_then(|hash| Self::str_or_empty(hash)),
            initial_supply: Token::parse_amount(token_entry.token_details.decimals as u32,
                                                &token_entry.token_details.genesis_or_mint_quantity)?,
            current_supply: Token::parse_amount(token_entry.token_details.decimals as u32,
                                                &token_entry.token_stats.qty_token_circulating_supply)?,
            block_created_height: token_entry.token_stats.block_created?,
        })
    }
}
