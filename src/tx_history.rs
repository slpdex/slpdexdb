use crate::tx_source::{tx_result, TxSource, TxFilter};
use crate::config::SLPDEXConfig;
use crate::token::Token;
use crate::db::Db;
use crate::slp_amount::SLPAmount;
use byteorder::{BigEndian, ReadBytesExt};
use std::io;
use std::collections::{HashSet, HashMap};
use cashcontracts::{Output, AddressType, Address};
use rug::Rational;

#[derive(Clone, Debug)]
pub struct TxHistory {
    pub txs: Vec<HistoricTx>,
    pub trade_offers: Vec<(usize, TradeOffer)>,
}

#[derive(Clone, Debug)]
pub struct HistoricTx {
    pub hash: [u8; 32],
    pub height: Option<i32>,
    pub timestamp: i64,
    pub tx_type: TxType,
    pub inputs: Vec<HistoricTxInput>,
    pub outputs: Vec<HistoricTxOutput>,
}

#[derive(Clone, Debug)]
pub enum SLPTxType {
    Genesis,
    Mint,
    Send,
    Commit,
}

#[derive(Clone, Debug)]
pub enum TxType {
    Default,
    SLP {
        token_hash: [u8; 32],
        token_type: i32,
        slp_type: SLPTxType,
    },
}

#[derive(Clone, Debug)]
pub enum OutputType {
    OpReturn,
    Unknown,
    Address(Address),
    Burned,
}

#[derive(Clone, Debug)]
pub struct HistoricTxOutput {
    pub value_satoshis: u64,
    pub value_token_base: SLPAmount,
    pub output: OutputType,
}

#[derive(Clone, Debug)]
pub struct HistoricTxInput {
    pub output_tx: [u8; 32],
    pub output_idx: i32,
    pub output: OutputType,
}

#[derive(Clone, Debug)]
pub struct TradeOffer {
    pub tx: [u8; 32],
    pub output_idx: Option<i32>,
    pub input_tx: [u8; 32],
    pub input_idx: i32,
    pub price_per_token: Rational,
    pub script_price: i64,
    pub is_inverted: bool,
    pub sell_amount_token_base: SLPAmount,
    pub receiving_address: Address,
}

impl SLPTxType {
    pub fn from_bytes(s: &[u8]) -> Option<Self> {
        match s {
            b"GENESIS" => Some(SLPTxType::Genesis),
            b"SEND"    => Some(SLPTxType::Send),
            b"MINT"    => Some(SLPTxType::Mint),
            b"COMMIT"  => Some(SLPTxType::Commit),
            _         => None,
        }
    }

    pub fn to_bytes(&self) -> &[u8] {
        match self {
            SLPTxType::Genesis => b"GENESIS",
            SLPTxType::Send    => b"SEND",
            SLPTxType::Mint    => b"MINT",
            SLPTxType::Commit  => b"COMMIT",
        }
    }
}

impl OutputType {
    pub fn address(&self) -> Option<&Address> {
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
            },
            OutputType::Burned => 4,
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
                    Address::from_cash_addr(addr.to_string())
                } else {
                    Address::from_cash_addr("bitcoincash:".to_string() + addr)
                };
                address.map(OutputType::Address).unwrap_or(OutputType::Unknown)
            },
            None => OutputType::Unknown
        }
    }

    pub fn from_entries(entries: &[tx_result::TxEntry],
                        now: i64,
                        config: &SLPDEXConfig) -> Self {
        let mut historic_txs = Vec::with_capacity(entries.len());
        let mut trade_offers = Vec::new();
        for entry in entries.iter() {
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
                            Some(slp) if i > 0 => {
                                let decimals = slp.detail.decimals as u32;
                                slp.detail.outputs
                                    .get(i - 1)
                                    .and_then(|output|
                                        SLPAmount::from_str_decimals(&output.amount, decimals).ok()
                                    )
                                    .unwrap_or(SLPAmount::new(0, decimals))
                            },
                            _ => SLPAmount::new(0, 0),
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
                        token_type: slp.detail.version_type,
                        token_hash: {
                            let mut token_id = [0; 32];
                            token_id.copy_from_slice(&hex::decode(&slp.detail.token_id).ok()?);
                            token_id
                        },
                        slp_type: SLPTxType::from_bytes(slp.detail.transaction_type.as_bytes())?,
                    }))
                    .unwrap_or(TxType::Default),
                inputs,
                outputs,
            };
            entry.slp.as_ref().and_then(|slp| {
                trade_offers.push((
                    historic_txs.len(),
                    TradeOffer::from_entry(&historic_tx, entry, config, slp.detail.decimals as u32)?,
                ));
                Some(())
            });
            historic_txs.push(historic_tx);
        }
        TxHistory {
            txs: historic_txs,
            trade_offers,
        }
    }

    pub fn _process_input_script(script: &cashcontracts::Script) -> OutputType {
        use cashcontracts::{Op, OpCodeType::*};
        let ops = script.ops();
        if ops.len() == 0 { return OutputType::Unknown; }
        if ops[0] == Op::Code(OpReturn) { return OutputType::OpReturn; }
        if ops.len() != 2 { return OutputType::Unknown }
        match &ops[1] {
            Op::Push(pubkey) if pubkey.len() == 33 => {
                OutputType::Address(
                    Address::from_serialized_pub_key("bitcoincash", AddressType::P2PKH, pubkey)
                )
            },
            _ => OutputType::Unknown
        }
    }

    pub fn _process_output_script(script: &cashcontracts::Script) -> OutputType {
        use cashcontracts::{Op::*, OpCodeType::*};
        match script.ops() {
            &[Code(OpDup), Code(OpHash160), Push(ref address), Code(OpEqualVerify), Code(OpCheckSig)] => {
                let mut address_bytes = [0; 20];
                address_bytes.copy_from_slice(address);
                OutputType::Address(Address::from_bytes(AddressType::P2PKH, address_bytes))
            },
            &[Code(OpHash160), Push(ref address), Code(OpEqual)] => {
                let mut address_bytes = [0; 20];
                address_bytes.copy_from_slice(address);
                OutputType::Address(Address::from_bytes(AddressType::P2SH, address_bytes))
            },
            ops if ops.len() > 0 && ops[0] == Code(OpReturn) => OutputType::OpReturn,
            _ => OutputType::Unknown
        }
    }

    pub fn _process_slp_output(script: &cashcontracts::Script, db: &Db)
            -> Option<(TxType, Vec<SLPAmount>, Token)> {
        use cashcontracts::{Op::*, OpCodeType::*, serialize};
        if !script.is_slp_safe() { return None; }
        let ops = script.ops();
        if ops.len() < 6 { return None; }
        match (&ops[0], &ops[1], &ops[2], &ops[3], &ops[4]) {
            (Code(OpReturn), Push(lokad_id), Push(token_type), Push(tx_type), Push(token_id))
                    if lokad_id == b"SLP\0" && token_type.len() <= 2 && token_id.len() == 32 => {
                let mut token_hash = [0; 32];
                token_hash.copy_from_slice(token_id);
                let token = Self::_fetch_token(&token_hash, db)?;
                let decimals = token.decimals as u32;
                let token_type = serialize::vec_to_int(token_type);
                let amounts = ops[5..].iter()
                    .map(|op| {
                        match op {
                            Push(vec) => Some(SLPAmount::from_slice(&vec, decimals).ok()?),
                            _ => None,
                        }
                    })
                    .collect::<Option<Vec<_>>>()?;
                if amounts.len() > 19 { return None }
                Some((
                    TxType::SLP {
                        slp_type: SLPTxType::from_bytes(tx_type)?,
                        token_type,
                        token_hash,
                    },
                    amounts,
                    token,
                ))
            },
            _ => { None }
        }
    }

    pub fn from_txs(txs: &[cashcontracts::Tx], now: i64, config: &SLPDEXConfig, db: &Db) -> Self {
        let mut historic_txs = Vec::new();
        let mut trade_offers = Vec::new();
        for tx in txs.iter() {
            let inputs = tx.inputs().iter()
                .map(|input| {
                    HistoricTxInput {
                        output_tx: input.outpoint.tx_hash.clone(),
                        output_idx: input.outpoint.vout as i32,
                        output: Self::_process_input_script(&input.script),
                    }
                })
                .collect::<Vec<_>>();
            let (tx_type, slp_amounts, token) = tx.outputs()
                .get(0)
                .and_then(|output| {
                    let (tx_type, slp_amounts, token) =
                        Self::_process_slp_output(&output.script, db)?;
                    Some((tx_type, slp_amounts, Some(token)))
                })
                .unwrap_or((TxType::Default, vec![], None));
            let outputs = tx.outputs().iter().enumerate()
                .map(|(output_idx, output)| {
                    HistoricTxOutput {
                        value_satoshis: output.value,
                        value_token_base: slp_amounts.get(output_idx).cloned()
                            .unwrap_or(SLPAmount::new(
                                0,
                                token.as_ref().map(|token| token.decimals as u32).unwrap_or(0),
                            )),
                        output: Self::_process_output_script(&output.script),
                    }
                })
                .chain(
                    slp_amounts.iter().skip(tx.outputs().len()).map(|amount| {
                        HistoricTxOutput {
                            value_satoshis: 0,
                            value_token_base: *amount,
                            output: OutputType::Burned,
                        }
                    })
                )
                .collect::<Vec<_>>();
            let historic_tx = HistoricTx {
                hash: tx.hash(),
                height: None,
                timestamp: now,
                tx_type,
                inputs,
                outputs,
            };
            let trade_offer = match &historic_tx.tx_type {
                TxType::SLP { .. } => token.and_then(
                    |token| TradeOffer::from_tx(&historic_tx, tx, config, &token)
                ),
                _ => None,
            };
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

    pub fn _fetch_token(token_hash: &[u8; 32], db: &Db) -> Option<Token> {
        match db.token(token_hash) {
            Ok(Some(token)) => Some(token),
            Ok(None) => {
                let mut token_entries = crate::token_source::TokenSource::new()
                    .request_tokens(&[TxFilter::TokenId(token_hash.clone())]).ok()?;
                println!("token entry: {:?}", token_entries);
                if token_entries.len() == 0 { return None }
                let token = Token::from_entry(token_entries.remove(0))?;
                println!("new token: {:?}", token);
                db.add_tokens(&[token.clone()]).unwrap();
                Some(token)
            },
            _ => None,
        }
    }

    pub fn validate_slp(&mut self, tx_source: &TxSource, _db: &Db, config: &SLPDEXConfig)
            -> reqwest::Result<()> {
        let tx_to_check = self.txs.iter()
            .flat_map(|tx| {
                tx.inputs.iter()
                    .map(|input| input.output_tx)
                    .take(match tx.tx_type {
                        TxType::SLP {..} => tx.inputs.len(),
                        TxType::Default => 0,
                    })
            })
            .collect::<HashSet<_>>();
        let tx_to_check = tx_to_check.into_iter()
            .map(TxFilter::TxHash)
            .collect::<Vec<_>>();
        if tx_to_check.len() == 0 { return Ok(()); }
        let validity_map = tx_source
            .request_slp_tx_validity(&tx_to_check, config)?
            .into_iter()
            .map(|validity| (cashcontracts::tx_hex_to_hash(&validity.tx.h), validity))
            .collect::<HashMap<_, _>>();
        for validity in validity_map.values() {
            println!("{}", serde_json::to_string(validity).unwrap_or(".".to_string()));
        }
        let mut txs = Vec::new();
        let mut trade_offers = Vec::new();
        for i in 0..self.txs.len() {
            let tx = self.txs.remove(0);
            let (token_hash, token_type) = match &tx.tx_type {
                TxType::SLP {token_hash, token_type, ..} => (token_hash, token_type),
                TxType::Default => {
                    txs.push(tx);
                    continue;
                },
            };
            println!("validating {}", cashcontracts::tx_hash_to_hex(&tx.hash));
            println!("token found: ");
            let decimals = tx.outputs.iter()
                .map(|output| output.value_token_base.decimals())
                .next();
            let output_sum = tx.outputs.iter()
                .map(|output| output.value_token_base)
                .sum::<SLPAmount>();
            let input_sum = tx.inputs.iter()
                .filter_map(|input| Some((input, validity_map.get(&input.output_tx)?)))
                .filter(|(tx_input, validity)|
                    validity.slp.valid &&
                        tx_input.output_idx > 0 &&
                        hex::decode(&validity.slp.detail.token_id).ok().as_ref()
                            .map(|t| t.as_slice()) == Some(token_hash) &&
                        validity.slp.detail.version_type == *token_type
                )
                .filter_map(|(tx_input, validity)|
                    validity.slp.detail.outputs.get((tx_input.output_idx - 1) as usize)
                )
                .filter_map(|slp_output: &tx_result::TxSLPOutput| {
                    Some(SLPAmount::from_str_decimals(&slp_output.amount, decimals?).ok()?)
                })
                .sum::<SLPAmount>();
            println!("input sum: {}", input_sum);
            println!("output sum: {}", output_sum);
            if input_sum >= output_sum {
                match self.trade_offers.iter()
                        .enumerate()
                        .find_map(|(j, (idx, _))| {
                            if *idx == i { Some(j) } else { None }
                        }) {
                    Some(idx) => trade_offers.push((txs.len(), self.trade_offers.remove(idx).1)),
                    None => {},
                };
                txs.push(tx);
            }
        }
        self.txs = txs;
        self.trade_offers = trade_offers;
        Ok(())
    }
}

struct _Price {
    script_price: u32,
    price_per_token: Rational,
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

    fn _decode_price(slp_decimals: i32, power_bytes: &[u8], price_bytes: &[u8]) -> Option<_Price> {
        let is_inverted = power_bytes.get(1) == Some(&1);
        let script_price = io::Cursor::new(price_bytes).read_u32::<BigEndian>().ok()?;
        let factor = Self::_FACTORS[slp_decimals as usize];
        let factor_rational = rug::Rational::from((factor, 1));
        let price_per_token = if is_inverted {
            rug::Rational::from((1, script_price)) * factor_rational
        } else {
            rug::Rational::from((script_price, 1)) * factor_rational
        };
        Some(_Price {
            script_price,
            price_per_token,
            power: *power_bytes.get(0)?,
            is_inverted,
        })
    }

    fn _contract_hash(output: &HistoricTxOutput,
                      price: &_Price,
                      tx_type: &TxType,
                      config: &SLPDEXConfig,
                      receiving_address: &cashcontracts::Address) -> Option<SLPAmount> {
        let (token_hash, token_type) = match tx_type {
            TxType::SLP {token_hash, token_type, ..} => (token_hash, *token_type),
            TxType::Default => return None,
        };
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
                    sell_amount_token: output.value_token_base.base_amount() as u64,
                    price: price.script_price,
                    dust_amount: config.dust_limit,
                    address: receiving_address.clone(),
                    fee_address: Some(config.fee_address.clone()),
                    fee_divisor: Some(config.fee_divisor.clone()),
                    spend_params: None,
                }.script().to_vec()
            );
            if address.bytes() == &hash {
                Some(output.value_token_base)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn from_entry(tx: &HistoricTx,
                      entry: &tx_result::TxEntry,
                      config: &SLPDEXConfig,
                      decimals: u32)
            -> Option<Self> {
        entry.inputs.iter().find_map(|input| {
            if input.b0 == tx_result::StackItem::Str(base64::encode("EXCH")) &&
                    input.b1 == (tx_result::StackItem::Op {op: 0x52}) {
                let price = entry.slp.as_ref()
                    .and_then(|slp| Self::_decode_price(
                        slp.detail.decimals,
                        &base64::decode(input.b2.get_str()?).ok()?,
                        &base64::decode(input.b3.get_str()?).ok()?,
                    ))?;
                let receiving_address = Address::from_slice(
                    AddressType::P2PKH,
                    &base64::decode(input.b4.get_str()?).ok()?,
                )?;
                let output_idx: i32 = 1;
                let contract_vals = tx.outputs.get(output_idx as usize)
                    .and_then(|output: &HistoricTxOutput| {
                        Some((
                            output_idx,
                            Self::_contract_hash(output,
                                                 &price,
                                                 &tx.tx_type,
                                                 config,
                                                 &receiving_address)?,
                        ))
                    });
                Some(TradeOffer {
                    tx: cashcontracts::tx_hex_to_hash(&entry.tx.h),
                    output_idx: contract_vals.map(|(idx, _)| idx),
                    input_tx: cashcontracts::tx_hex_to_hash(&input.e.h),
                    input_idx: input.e.i,
                    price_per_token: price.price_per_token,
                    is_inverted: price.is_inverted,
                    script_price: price.script_price as i64,
                    sell_amount_token_base: contract_vals
                        .map(|(_, amount)| amount)
                        .unwrap_or(SLPAmount::new(0, decimals)),
                    receiving_address,
                })
            } else {
                None
            }
        })
    }

    pub fn from_tx(historic_tx: &HistoricTx,
                   tx: &cashcontracts::Tx,
                   config: &SLPDEXConfig,
                   token: &Token) -> Option<Self> {
        use cashcontracts::{Op::*, OpCodeType::*};
        println!("validating trade offer");
        let (token_hash, token_type) = match &historic_tx.tx_type {
            TxType::SLP {token_hash, token_type, ..} => (token_hash, *token_type),
            TxType::Default => return { println!("isnt slp"); None },
        };
        tx.inputs().iter().find_map(|input| {
            let ops = input.script.ops();
            if ops.len() < 5 { return None; }
            match &input.script.ops()[..5] {
                &[Push(ref exch), Code(Op2), Push(ref power), Push(ref price), Push(ref address)]
                        if exch.as_slice() == config.exch_lokad.as_bytes() => {
                    let price = Self::_decode_price(token.decimals, power, price)?;
                    println!("succeed price decoding");
                    let receiving_address = Address::from_slice(
                        AddressType::P2PKH,
                        address,
                    )?;
                    println!("succeed address decoding");
                    let output_idx: i32 = 1;
                    let contract_vals = historic_tx.outputs.get(output_idx as usize)
                        .and_then(|output: &HistoricTxOutput| {
                            Some((
                                output_idx,
                                Self::_contract_hash(output,
                                                     &price,
                                                     &historic_tx.tx_type,
                                                     config,
                                                     &receiving_address)?,
                            ))
                        });
                    println!("contract vals {:?}", contract_vals);
                    Some(TradeOffer {
                        tx: historic_tx.hash.clone(),
                        output_idx: contract_vals.map(|(idx, _)| idx),
                        input_tx: input.outpoint.tx_hash.clone(),
                        input_idx: input.outpoint.vout as i32,
                        price_per_token: price.price_per_token,
                        is_inverted: price.is_inverted,
                        script_price: price.script_price as i64,
                        sell_amount_token_base: contract_vals
                            .map(|(_, amount)| amount)
                            .unwrap_or(SLPAmount::new(0, token.decimals as u32)),
                        receiving_address,
                    })
                }
                _ => { println!("bad stack {}", input.script); None }
            }
        })
    }
}
