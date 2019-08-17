use cashcontracts::{Address, tx_hash_to_hex};
use json::{JsonValue, object, array};
use slpdexdb_base::SLPDEXConfig;
use crate::endpoint::Endpoint;


#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TxFilter {
    Address(Address),
    TokenId([u8; 32]),
    MinBlockHeight(i32),
    MinTxHash([u8; 32]),
    TxHash([u8; 32]),
    Exch,
    SortBy(SortKey),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SortKey {
    TxHash,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TxResultKind {
    Complete,
    SLPValidity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Confirmedness {
    Confirmed,
    Unconfirmed,
    Both,
}

pub struct TxSource {
    endpoint: Endpoint
}

pub mod tx_result {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Blk {
        pub t: u64,
        pub i: i32,
    }
    #[derive(Serialize, Deserialize, Debug)]
    pub struct Tx {
        pub h: String,
    }
    #[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
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
    #[derive(Serialize, Deserialize, Debug)]
    pub struct TxInputEdge {
        pub a: Option<String>,
        pub h: String,
        pub i: i32,
    }
    #[derive(Serialize, Deserialize, Debug)]
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
    #[derive(Serialize, Deserialize, Debug)]
    pub struct TxOutputEdge {
        pub v: u64,
        pub a: Option<String>,
    }
    #[derive(Serialize, Deserialize, Debug)]
    pub struct TxOutput {
        pub e: TxOutputEdge,
        #[serde(default)]
        pub b0: StackItem,
    }
    #[derive(Serialize, Deserialize, Debug)]
    pub struct TxSLP {
        pub valid: bool,
        pub detail: TxSLPDetail
    }
    #[derive(Serialize, Deserialize, Debug)]
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
    #[derive(Serialize, Deserialize, Debug)]
    pub struct TxSLPOutput {
        pub address: String,
        pub amount: String,
    }
    #[derive(Serialize, Deserialize, Debug)]
    pub struct TxEntry {
        pub blk: Option<Blk>,
        pub tx: Tx,
        #[serde(rename = "in")]
        pub inputs: Vec<TxInput>,
        #[serde(rename = "out")]
        pub outputs: Vec<TxOutput>,
        pub slp: Option<TxSLP>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct TxSLPValidity {
        pub tx: Tx,
        pub slp: TxSLP,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct TxResult {
        pub u: Option<Vec<TxEntry>>,
        pub c: Option<Vec<TxEntry>>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct TxSLPValidityResult {
        pub u: Option<Vec<TxSLPValidity>>,
        pub c: Option<Vec<TxSLPValidity>>,
    }
}

impl TxFilter {
    pub fn slp_conditions(filters: &[TxFilter],
                          config: &SLPDEXConfig) -> Vec<(&'static str, JsonValue)> {
        let is_exch = filters.iter().any(|filter| filter == &TxFilter::Exch);
        let addresses = filters.iter()
            .filter_map(|filter| {
                match filter {
                    TxFilter::Address(addr) if is_exch =>
                        Some(base64::encode(addr.bytes())),
                    TxFilter::Address(addr) if !is_exch =>
                        Some(addr.with_prefix("simpleledger".to_string()).cash_addr().to_string()),
                    _ => None,
                }
            })
            .map(JsonValue::String)
            .collect::<Vec<_>>();
        filters.iter()
            .flat_map(|filter| {
                match filter {
                    TxFilter::Exch => vec![
                        ("in.b0", JsonValue::String(config.exch_lokad_b64.to_string())),
                        ("in.b1", object!{"op" => 0x50 + config.exch_version}),
                    ],
                    TxFilter::TokenId(token_id) => vec![
                        ("slp.detail.tokenIdHex", JsonValue::String(tx_hash_to_hex(token_id)))
                    ],
                    _ => vec![],
                }
            })
            .chain(
                if addresses.len() > 0 {
                    if is_exch {
                        vec![
                            ("in.b4", object!{"$in" => JsonValue::Array(addresses)}),
                        ]
                    } else {
                        vec![
                            ("$or", array![
                                object!{"out.e.a" => object!{"$in" => JsonValue::Array(addresses.clone())}},
                                object!{"in.e.a" => object!{"$in" => JsonValue::Array(addresses)}}
                            ]),
                        ]
                    }
                } else {
                    vec![]
                }
            )
            .chain(vec![("slp.valid", JsonValue::Boolean(true))])
            .collect()
    }

    pub fn bch_conditions(filters: &[TxFilter]) -> Vec<(&'static str, JsonValue)> {
        let base_address_list = filters.iter()
            .filter_map(|filter| {
                match filter {
                    TxFilter::Address(addr) => {
                        let prefix = "bitcoincash";
                        let addr = addr.with_prefix(prefix.to_string());
                        Some(addr.cash_addr()[prefix.len() + 1..].to_string())
                    }
                    _ => None,
                }
            })
            .map(JsonValue::String)
            .collect::<Vec<_>>();
        let mut result = vec![
            ("out.b1", object!{"$ne" => JsonValue::String(base64::encode(b"SLP\0"))}),
        ];
        if base_address_list.len() > 0 {
            result.push(("out.e.a", object!{"$in" => JsonValue::Array(base_address_list)}));
        }
        result
    }

    pub fn base_conditions(filters: &[TxFilter]) -> Vec<(&'static str, JsonValue)> {
        let tx_hashes = filters.iter()
            .filter_map(|filter| {
                match filter {
                    TxFilter::TxHash(tx_hash) => Some(
                        JsonValue::String(cashcontracts::tx_hash_to_hex(tx_hash))
                    ),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        filters.iter()
            .filter_map(|filter| {
                match filter {
                    TxFilter::MinBlockHeight(height) => Some(("$or", array![
                        object!{"blk"   => object!{"$exists" => false}},
                        object!{"blk.i" => object!{"$gte" => *height}}
                    ])),
                    TxFilter::MinTxHash(tx_hash) => Some(
                        ("tx.h", object!{"$gt" => cashcontracts::tx_hash_to_hex(tx_hash)})
                    ),
                    _ => None,
                }
            })
            .chain(if tx_hashes.len() > 0 {
                vec![("tx.h", object!{"$in" => JsonValue::Array(tx_hashes)})]
            } else {
                vec![]
            })
            .collect()

    }

    pub fn sort_by(filters: &[TxFilter]) -> JsonValue {
        filters.iter()
            .find_map(|filter| match filter {
                TxFilter::SortBy(sort) => {
                    match sort {
                        SortKey::TxHash => Some(object!{"tx.h" => 1}),
                    }
                },
                _ => None,
            })
            .unwrap_or(object!{})
    }
}

impl TxSource {
    pub fn new() -> Self {
        TxSource {
            endpoint: Endpoint::new(),
        }
    }

    fn _query(&self,
              endpoint_url: &str,
              conditions: Vec<(&'static str, JsonValue)>,
              sort: JsonValue,
              result_kind: TxResultKind,
              confirmedness: Confirmedness) -> reqwest::Result<String> {
        let mut conditions_json = Vec::new();
        for (key, json) in conditions {
            conditions_json.push(object!{key => json});
        }
        let db = match confirmedness {
            Confirmedness::Unconfirmed => array!["u"],
            Confirmedness::Confirmed   => array!["c"],
            Confirmedness::Both        => array!["u", "c"],
        };
        let mut query = object!{
            "v" => 3,
            "q" => object!{
                "db" => db,
                "find" => object!{"$and" => JsonValue::Array(conditions_json)},
                "sort" => sort,
            },
        };
        if result_kind == TxResultKind::SLPValidity {
            query["r"] = object!{"f" => "[.[] | {tx: .tx, slp: .slp} ]"};
        }
        let query_json = json::stringify(query);
        println!("{}", query_json);
        let query_b64 = base64::encode(&query_json);
        reqwest::get(&format!("{}{}", endpoint_url, query_b64))?.text()
        //println!("{}", text);
        //Ok(serde_json::from_str::<R>(&text).unwrap())
    }

    fn _request_txs(&self,
                    filters: &[TxFilter],
                    config: &SLPDEXConfig,
                    result_kind: TxResultKind,
                    confirmedness: Confirmedness)
            -> reqwest::Result<Vec<String>> {
        let slp_only =
            filters.iter().any(|f| match f {
                TxFilter::TokenId(_) | TxFilter::Exch => true,
                _ => false,
            }) || result_kind == TxResultKind::SLPValidity;
        let base_conditions = TxFilter::base_conditions(filters);
        let sort = TxFilter::sort_by(filters);
        let mut results = Vec::new();
        if !slp_only {
            let mut bch_conditions = TxFilter::bch_conditions(filters);
            bch_conditions.append(&mut base_conditions.clone());
            results.push(
                self._query(
                    &self.endpoint.bitdb_endpoint_url,
                    bch_conditions,
                    sort.clone(),
                    result_kind,
                    confirmedness,
                )?
            );
        }
        let mut slp_conditions = TxFilter::slp_conditions(filters, config);
        slp_conditions.append(&mut base_conditions.clone());
        results.push(
            self._query(
                &self.endpoint.slpdb_endpoint_url,
                slp_conditions,
                sort,
                result_kind,
                confirmedness,
            )?
        );
        Ok(results)
    }

    pub fn request_txs(&self, filters: &[TxFilter], config: &SLPDEXConfig, confirmedness: Confirmedness)
            -> reqwest::Result<Vec<tx_result::TxEntry>> {
        let results_json = self._request_txs(filters, config, TxResultKind::Complete, confirmedness)?;
        let mut results = Vec::new();
        for result_json in results_json {
            let result = serde_json::from_str::<tx_result::TxResult>(&result_json).unwrap();
            result.c.map(|mut r| results.append(&mut r));
            result.u.map(|mut r| results.append(&mut r));
        }
        Ok(results)
    }

    pub fn request_slp_tx_validity(&self, filters: &[TxFilter], config: &SLPDEXConfig,
                                   confirmedness: Confirmedness)
            -> reqwest::Result<Vec<tx_result::TxSLPValidity>> {
        let results_json = self._request_txs(filters, config, TxResultKind::Complete, confirmedness)?;
        let mut results = Vec::new();
        for result_json in results_json {
            let result = serde_json::from_str::<tx_result::TxSLPValidityResult>(&result_json)
                .unwrap();
            result.c.map(|mut r| results.append(&mut r));
            result.u.map(|mut r| results.append(&mut r));
        }
        Ok(results)
    }
}
