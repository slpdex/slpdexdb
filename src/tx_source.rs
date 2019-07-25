use cashcontracts::Address;
use itertools::Itertools;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TxFilter {
    Address(Address),
    TokenId([u8; 32]),
    Exch,
}

pub struct TxSource {
    bitdb_endpoint_url: String,
    slpdb_endpoint_url: String,
}

pub mod tx_result {
    use serde::Deserialize;
    #[derive(Deserialize, Debug)]
    pub struct Blk {
        pub t: u64,
        pub i: i32,
    }
    #[derive(Deserialize, Debug)]
    pub struct Tx {
        pub h: String,
    }
    #[derive(Deserialize, Debug, Eq, PartialEq)]
    #[serde(untagged)]
    pub enum StackItem {
        Str(String),
        Op { op: i32 },
        Undefined,
    }
    impl Default for StackItem {
        fn default() -> Self { StackItem::Undefined }
    }
    impl StackItem {
        pub fn get_str(&self) -> Option<&str> {
            match self {
                StackItem::Str(str) => Some(str),
                _ => None,
            }
        }
    }
    #[derive(Deserialize, Debug)]
    pub struct TxInputEdge {
        pub a: Option<String>,
        pub h: String,
        pub i: i32,
    }
    #[derive(Deserialize, Debug)]
    pub struct TxInput {
        pub e: TxInputEdge,
        #[serde(default)]
        pub b0: StackItem,
        #[serde(default)]
        pub b1: StackItem,
        #[serde(default)]
        pub b2: StackItem,
        #[serde(default)]
        pub b3: StackItem,
        #[serde(default)]
        pub b4: StackItem,
        #[serde(default)]
        pub b5: StackItem,
        #[serde(default)]
        pub b6: StackItem,
        #[serde(default)]
        pub b7: StackItem,
    }
    #[derive(Deserialize, Debug)]
    pub struct TxOutputEdge {
        pub v: u64,
        pub a: Option<String>,
    }
    #[derive(Deserialize, Debug)]
    pub struct TxOutput {
        pub e: TxOutputEdge,
        #[serde(default)]
        pub b0: StackItem,
    }
    #[derive(Deserialize, Debug)]
    pub struct TxSLP {
        pub valid: bool,
        pub detail: TxSLPDetail
    }
    #[derive(Deserialize, Debug)]
    pub struct TxSLPDetail {
        pub decimals: i32,
        #[serde(rename = "tokenIdHex")]
        pub token_id: String,
        #[serde(rename = "transactionType")]
        pub transaction_type: String,
        #[serde(rename = "versionType")]
        pub version_type: i32,
        pub outputs: Vec<TxSLPOutput>,
    }
    #[derive(Deserialize, Debug)]
    pub struct TxSLPOutput {
        pub address: String,
        pub amount: String,
    }
    #[derive(Deserialize, Debug)]
    pub struct TxEntry {
        pub blk: Option<Blk>,
        pub tx: Tx,
        #[serde(rename = "in")]
        pub inputs: Vec<TxInput>,
        #[serde(rename = "out")]
        pub outputs: Vec<TxOutput>,
        pub slp: Option<TxSLP>,
    }
    #[derive(Deserialize, Debug)]
    pub struct TxResult {
        pub u: Vec<TxEntry>,
        pub c: Vec<TxEntry>,
    }
}

impl TxSource {
    pub fn new() -> Self {
        TxSource {
            bitdb_endpoint_url: "https://bitdb.bch.sx/q/".to_string(),
            slpdb_endpoint_url: "https://slpdb.fountainhead.cash/q/".to_string(),
        }
    }

    fn _request_slp_conditions(&self, filters: &[TxFilter]) -> String {
        let is_exch = filters.iter().any(|f| f == &TxFilter::Exch);
        let slp_address_list = filters.iter().filter_map(|f| {
            match f {
                TxFilter::Address(addr) => {
                    if is_exch {
                        Some(format!("\"{}\"", base64::encode(addr.bytes())))
                    } else {
                        Some(
                            format!("\"{}\"",
                                    addr.with_prefix("simpleledger".to_string()).cash_addr())
                        )
                    }
                }
                _ => None,
            }
        }).join(",\n");
        let conditions = filters.iter().filter_map(|f| {
            match f {
                TxFilter::Exch => Some(format!(
                    r#""in.b0": "{}",
                       "in.b1": {{"op": {}}}"#,
                    base64::encode("EXCH"),
                    0x52,
                )),
                TxFilter::TokenId(token_id) => Some(format!(
                    "\"slp.detail.tokenIdHex\": \"{}\"",
                    hex::encode(token_id),
                )),
                _ => None,
            }
        }).chain(
            if slp_address_list.len() > 0 {
                if is_exch {
                    vec![format!("\"in.b4\": {{\"$in\": [{}]}}", slp_address_list)]
                } else {
                    vec![format!("\"out.e.a\": {{\"$in\": [{}]}}", slp_address_list),
                         format!("\"in.e.a\":  {{\"$in\": [{}]}}", slp_address_list)]
                }
            } else {
                vec![]
            }
        ).chain(vec!["\"slp.valid\": true".to_string()])
            .join(",\n");
        return conditions
    }

    fn _request_bch_conditions(&self, filters: &[TxFilter]) -> String {
        let base_address_list = filters.iter().filter_map(|f| {
            match f {
                TxFilter::Address(addr) => {
                    let prefix = "bitcoincash";
                    let addr = addr.with_prefix(prefix.to_string());
                    Some(
                        format!("\"{}\"", &addr.cash_addr()[prefix.len() + 1..])
                    )
                }
                _ => None,
            }
        }).join(",\n");
        format!(r#""out.e.a": {{"$in": [{}]}},
                  "out.b1": {{"$ne": "{}"}}"#,
                base_address_list,
                base64::encode(b"SLP\0"))
    }

    fn _query(&self, endpoint_url: &str, conditions: &str) -> reqwest::Result<tx_result::TxResult> {
        let query_json = format!(r#"
            {{
              "v": 3,
              "q": {{
                "db": ["u", "c"],
                "find": {{
                  {}
                }}
              }}
            }}
        "#, conditions);
        println!("query_json={}", query_json);
        let query_b64 = base64::encode(&query_json);
        let text = reqwest::get(&format!("{}{}", endpoint_url, query_b64))?.text()?;
        println!("{}", text);
        Ok(serde_json::from_str(&text).unwrap())
    }

    pub fn request_txs(&self, filters: &[TxFilter]) {
        let slp_only = filters.iter().any(|f| match f {
            TxFilter::TokenId(_) | TxFilter::Exch => true,
            _ => false,
        });
        if !slp_only {
            let bch_result = self._query(
                &self.bitdb_endpoint_url,
                &self._request_bch_conditions(filters),
            ).unwrap();
            dbg!(bch_result);
        }
        let slp_result = self._query(
            &self.slpdb_endpoint_url,
            &self._request_slp_conditions(filters),
        ).unwrap();
        dbg!(slp_result);
    }
}
