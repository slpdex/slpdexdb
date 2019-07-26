use crate::tx_source::tx_result;
use crate::config::SLPDEXConfig;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use std::io;
use cashcontracts::{Output, AddressType};

pub struct TxHistory {
    pub txs: Vec<HistoricTx>,
    pub trade_offers: Vec<(usize, TradeOffer)>,
}

pub struct HistoricTx {
    pub hash: [u8; 32],
    pub height: Option<i32>,
    pub timestamp: i64,
    pub tx_type: TxType,
    pub inputs: Vec<HistoricTxInput>,
    pub outputs: Vec<HistoricTxOutput>,
}

pub enum SLPTxType {
    Genesis,
    Mint,
    Send,
    Commit,
}

pub enum TxType {
    Default,
    SLP {
        token_hash: [u8; 32],
        version: i32,
        slp_type: SLPTxType,
    },
}

pub enum OutputType {
    OpReturn,
    Unknown,
    Address(cashcontracts::Address),
}

pub struct HistoricTxOutput {
    pub value_satoshis: u64,
    pub value_token_base: u64,
    pub output: OutputType,
}

pub struct HistoricTxInput {
    pub output_tx: [u8; 32],
    pub output_idx: i32,
    pub output: OutputType,
}

pub struct TradeOffer {
    pub tx: [u8; 32],
    pub output_idx: Option<i32>,
    pub input_tx: [u8; 32],
    pub input_idx: i32,
    pub approx_price_per_token: f64,
    pub price_per_token_numer: i64,
    pub price_per_token_denom: i64,
    pub script_price: i64,
    pub sell_amount_token_base: i64,
    pub receiving_address: cashcontracts::Address,
}

impl SLPTxType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "GENESIS" => Some(SLPTxType::Genesis),
            "SEND"    => Some(SLPTxType::Send),
            "MINT"    => Some(SLPTxType::Mint),
            "COMMIT"  => Some(SLPTxType::Commit),
            _         => None,
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            SLPTxType::Genesis => "GENESIS",
            SLPTxType::Send    => "SEND",
            SLPTxType::Mint    => "MINT",
            SLPTxType::Commit  => "COMMIT",
        }
    }
}

impl OutputType {
    pub fn address(&self) -> Option<&cashcontracts::Address> {
        match self {
            OutputType::Address(addr) => Some(addr),
            _ => None,
        }
    }

    pub fn id(&self) -> i32 {
        match self {
            OutputType::Unknown => 0,
            OutputType::OpReturn => 1,
            OutputType::Address(addr) => match addr.addr_type() {
                AddressType::P2SH => 2,
                AddressType::P2PKH => 3,
            }
        }
    }
}

impl TxType {
    pub fn id(&self) -> i32 {
        match self {
            TxType::Default => 1,
            TxType::SLP {..} => 2,
        }
    }
}

impl TxHistory {
    fn _process_address(address: &Option<String>) -> OutputType {
        match address {
            Some(addr) => {
                let address = if addr.contains(":") {
                    cashcontracts::Address::from_cash_addr(addr.to_string())
                } else {
                    cashcontracts::Address::from_cash_addr("bitcoincash:".to_string() + addr)
                };
                address.map(OutputType::Address).unwrap_or(OutputType::Unknown)
            },
            None => OutputType::Unknown
        }
    }

    fn _parse_slp_amount(amount: &str, decimals: i32) -> u64 {
        let prec = 128;
        rug::Float::parse(amount).map(|f| {
            let val = rug::Float::with_val(prec, f);
            let factor = rug::Float::with_val(prec, rug::Float::u_pow_u(10, decimals as u32));
            (val * &factor).to_integer().map(|i| i.to_u128().unwrap_or(0)).unwrap_or(0) as u64
        }).unwrap_or(0)
    }

    pub fn from_entries(entries: &[tx_result::TxEntry],
                        now: i64,
                        config: &SLPDEXConfig) -> Self {
        let mut historic_txs = Vec::with_capacity(entries.len());
        let mut trade_offers = Vec::new();
        for (idx, entry) in entries.iter().enumerate() {
            let inputs = entry.inputs.iter()
                .map(|input| {
                    HistoricTxInput {
                        output_tx: cashcontracts::tx_hex_to_hash(&input.e.h),
                        output_idx: input.e.i,
                        output: Self::_process_address(&input.e.a)
                    }
                })
                .collect::<Vec<_>>();
            let outputs = entry.outputs.iter()
                .enumerate()
                .map(|(i, output)| {
                    HistoricTxOutput {
                        value_satoshis: output.e.v,
                        value_token_base: match &entry.slp {
                            Some(slp) if i > 0 => slp.detail.outputs.get(i - 1).map(
                                |output| Self::_parse_slp_amount(&output.amount, slp.detail.decimals)
                            ).unwrap_or(0),
                            _ => 0,
                        },
                        output: if output.b0 == (tx_result::StackItem::Op {op: 0x6a}) {
                            OutputType::OpReturn
                        } else {
                            Self::_process_address(&output.e.a)
                        },
                    }
                })
                .collect::<Vec<_>>();
            let historic_tx = HistoricTx {
                hash: cashcontracts::tx_hex_to_hash(&entry.tx.h),
                height: entry.blk.as_ref().map(|blk| blk.i),
                timestamp: entry.blk.as_ref().map(|blk| blk.t as i64).unwrap_or(now),
                tx_type: entry.slp.as_ref()
                    .and_then(|slp| Some(TxType::SLP {
                        version: slp.detail.version_type,
                        token_hash: {
                            let mut token_id = [0; 32];
                            token_id.copy_from_slice(&hex::decode(&slp.detail.token_id).ok()?);
                            token_id
                        },
                        slp_type: SLPTxType::from_str(&slp.detail.transaction_type)?,
                    }))
                    .unwrap_or(TxType::Default),
                inputs,
                outputs,
            };
            let trade_offer = TradeOffer::from_entry(&historic_tx, entry, config);
            match trade_offer {
                Some(trade_offer) => trade_offers.push((historic_txs.len(), trade_offer)),
                None => {},
            }
            historic_txs.push(historic_tx);
        }
        TxHistory {
            txs: historic_txs,
            trade_offers,
        }
    }
}

struct _Price {
    script_price: u32,
    approx_price_per_token: f64,
    price_per_token_numer: i64,
    price_per_token_denom: i64,
    power: u8,
    is_inverted: bool,
}

impl TradeOffer {
    const _FACTORS: [u64; 10] = [
        1,
        10,
        100,
        1_000,
        10_000,
        100_000,
        1_000_000,
        10_000_000,
        100_000_000,
        1_000_000_000,
    ];

    fn _decode_price(slp_decimals: i32, power_b64: &str, price_b64: &str) -> Option<_Price> {
        let power_bytes = base64::decode(power_b64).ok()?;
        let mut price_bytes = io::Cursor::new(base64::decode(price_b64).ok()?);
        let is_inverted = power_bytes.get(1) == Some(&1);
        let script_price = price_bytes.read_u32::<BigEndian>().ok()?;
        let factor = Self::_FACTORS[slp_decimals as usize];
        let factor_rational = rug::Rational::from((factor, 1));
        let approx_price_per_token = if is_inverted {
            (1.0 / (script_price as f64)) * (factor as f64)
        } else {
            (script_price as f64) * (factor as f64)
        };
        let price_per_token = if is_inverted {
            rug::Rational::from((1, script_price)) * factor_rational
        } else {
            rug::Rational::from((script_price, 1)) * factor_rational
        };
        Some(_Price {
            script_price,
            approx_price_per_token,
            price_per_token_numer: price_per_token.numer().to_i128()? as i64,
            price_per_token_denom: price_per_token.denom().to_i128()? as i64,
            power: *power_bytes.get(0)?,
            is_inverted,
        })
    }

    pub fn from_entry(tx: &HistoricTx, entry: &tx_result::TxEntry, config: &SLPDEXConfig)
            -> Option<Self> {
        entry.inputs.iter().enumerate().find_map(|(i, input)| {
            if input.b0 == tx_result::StackItem::Str(base64::encode("EXCH")) &&
                    input.b1 == (tx_result::StackItem::Op {op: 0x52}) {
                let (token_hash, token_type, slp_type) = match &tx.tx_type {
                    TxType::SLP {token_hash, version, slp_type} => (token_hash, *version, slp_type),
                    TxType::Default => return None,
                };
                let price = entry.slp.as_ref()
                    .and_then(|slp| Self::_decode_price(
                        slp.detail.decimals,
                        input.b2.get_str()?,
                        input.b3.get_str()?,
                    ))?;
                let receiving_address = cashcontracts::Address::from_bytes(
                    AddressType::P2PKH,
                    {
                        let mut payload = [0; 20];
                        payload.copy_from_slice(&base64::decode(input.b4.get_str()?).ok()?);
                        payload
                    },
                );
                let output_idx: i32 = 1;
                let contract_vals = tx.outputs.get(output_idx as usize)
                    .and_then(|output: &HistoricTxOutput| {
                        let address = output.output.address()?;
                        if address.addr_type() == AddressType::P2SH {
                            let hash = cashcontracts::hash160(
                                &cashcontracts::AdvancedTradeOffer {
                                    value: output.value_satoshis,
                                    lokad_id: b"EXCH".to_vec(),
                                    version: 2,
                                    power: price.power,
                                    is_inverted: price.is_inverted,
                                    token_id: token_hash.clone(),
                                    token_type: token_type as u8,
                                    sell_amount_token: output.value_token_base,
                                    price: price.script_price,
                                    dust_amount: config.dust_limit,
                                    address: receiving_address.clone(),
                                    fee_address: Some(config.fee_address.clone()),
                                    fee_divisor: Some(config.fee_divisor.clone()),
                                    spend_params: None,
                                }.script().to_vec()
                            );
                            if address.bytes() == &hash {
                                Some((output_idx, output.value_token_base))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    });
                Some(TradeOffer {
                    tx: cashcontracts::tx_hex_to_hash(&entry.tx.h),
                    output_idx: contract_vals.map(|(idx, _)| idx),
                    input_tx: cashcontracts::tx_hex_to_hash(&input.e.h),
                    input_idx: input.e.i,
                    approx_price_per_token: price.approx_price_per_token,
                    price_per_token_numer: price.price_per_token_numer,
                    price_per_token_denom: price.price_per_token_denom,
                    script_price: price.script_price as i64,
                    sell_amount_token_base: contract_vals
                        .map(|(_, amount)| amount)
                        .unwrap_or(0) as i64,
                    receiving_address,
                })
            } else {
                None
            }
        })
    }
}
