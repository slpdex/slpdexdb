use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::block::BlockHeader;
use crate::tx_history::{TxHistory, TxType};
use crate::update_history::{UpdateHistory, UpdateSubjectType};
use crate::token::Token;
use crate::{models, schema::*};
use crate::slp_amount::SLPAmount;
use crate::convert_numeric::rational_to_pg_numeric;

use std::collections::{HashMap, HashSet, BTreeSet};

const PRICE_DIGITS: u16 = 26;

pub struct Db {
    connection: PgConnection,
}

impl Db {
    pub fn new(connection: PgConnection) -> Self {
        Db { connection }
    }

    pub fn add_headers(&self, headers: &[BlockHeader]) -> QueryResult<()> {
        let mut remaining_visit = (0..headers.len()).into_iter().collect::<BTreeSet<_>>();
        let mut heights = self.header_tips(10)?
            .iter()
            .map(|(header, height)| (header.hash(), *height))
            .collect::<HashMap<_, _>>();
        loop {
            let tuple = remaining_visit
                .iter().cloned()
                .find_map(|i|
                    if headers[i].prev_block != [0; 32] {
                        heights.get(&headers[i].prev_block).map(|i| *i + 1)
                    } else {
                        Some(0)
                    }.map(|height| (i, &headers[i], height))
                );
            match tuple {
                Some((i, header, height)) => {
                    heights.insert(header.hash(), height);
                    remaining_visit.remove(&i);
                },
                None => break,
            };
        }
        let db_blocks = headers.iter().map(|header| {
            models::Block::from_block_header(header, heights[&header.hash()])
        }).collect::<Vec<_>>();
        diesel::insert_into(blocks::table)
            .values(&db_blocks)
            .execute(&self.connection)?;
        Ok(())
    }

    pub fn header_tips(&self, n_recent: i64) -> QueryResult<Vec<(BlockHeader, i32)>> {
        Ok(blocks::table
            .order(blocks::height.desc())
            .limit(n_recent)
            .load::<models::Block>(&self.connection)?
            .into_iter()
            .map(|block: models::Block| (block.to_block_header(), block.height))
            .collect())
    }

    pub fn header_tip(&self) -> QueryResult<Option<(BlockHeader, i32)>> {
        let mut tips = self.header_tips(1)?;
        if tips.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(tips.remove(0)))
        }
    }

    pub fn add_tx_history(&self, tx_history: &TxHistory) -> QueryResult<()> {
        self.connection.transaction(|| {
            let token_hashes = tx_history.txs.iter()
                .filter_map(|tx| {
                    match tx.tx_type {
                        TxType::SLP {token_hash, ..} => Some(token_hash.to_vec()),
                        TxType::Default => None,
                    }
                })
                .collect::<HashSet<_>>();
            let new_txs = tx_history.txs.iter().map(|tx| {
                models::NewTx {
                    hash: tx.hash.to_vec(),
                    height: tx.height,
                    tx_type: tx.tx_type.id(),
                    timestamp: tx.timestamp,
                }
            }).collect::<Vec<_>>();
            let tokens: Vec<(Vec<u8>, i32)> = token::table
                .select((token::hash, token::id))
                .filter(token::hash.eq_any(token_hashes))
                .load(&self.connection)?;
            let token_ids = tokens.into_iter().collect::<HashMap<_, _>>();
            let tx_ids = diesel::insert_into(tx::table)
                .values(&new_txs)
                .on_conflict(tx::hash)
                .do_update().set((tx::height.eq(tx::height),
                                  tx::tx_type.eq(tx::tx_type),
                                  tx::timestamp.eq(tx::timestamp)))
                .returning(tx::id)
                .get_results::<i64>(&self.connection)?;
            let new_slp_txs = tx_history.txs
                .iter()
                .zip(tx_ids.iter().cloned())
                .filter_map(|(tx, id)| {
                    match &tx.tx_type {
                        TxType::SLP {token_hash, token_type, slp_type} => Some(models::SlpTx {
                            tx: id,
                            slp_type: String::from_utf8_lossy(slp_type.to_bytes()).to_string(),
                            token: *token_ids.get(token_hash.as_ref())?,
                            version: *token_type,
                        }),
                        TxType::Default => None,
                    }
                }).collect::<Vec<_>>();
            diesel::insert_into(slp_tx::table)
                .values(&new_slp_txs)
                .on_conflict_do_nothing()
                .execute(&self.connection)?;
            let new_outputs = tx_history.txs.iter()
                .zip(tx_ids.iter().cloned())
                .flat_map(|(tx, id)| {
                    tx.outputs.iter().enumerate().map(move |(output_idx, output)|
                        models::TxOutput {
                            tx: id,
                            idx: output_idx as i32,
                            value_satoshis: output.value_satoshis as i64,
                            value_token_base: output.value_token_base.into(),
                            address: output.output.address()
                                .map(|addr| addr.bytes().to_vec()),
                            output_type: output.output.id(),
                        })
                })
                .collect::<Vec<_>>();
            diesel::insert_into(tx_output::table)
                .values(&new_outputs)
                .on_conflict_do_nothing()
                .execute(&self.connection)?;
            let new_inputs = tx_history.txs.iter()
                .zip(tx_ids.iter().cloned())
                .flat_map(|(tx, id)| {
                    tx.inputs.iter().enumerate().map(move |(input_idx, input)| {
                        models::TxInput {
                            tx: id,
                            idx: input_idx as i32,
                            output_tx: input.output_tx.to_vec(),
                            output_idx: input.output_idx,
                            address: input.output.address()
                                .map(|addr| addr.bytes().to_vec()),
                        }
                    })
                })
                .collect::<Vec<_>>();
            diesel::insert_into(tx_input::table)
                .values(&new_inputs)
                .on_conflict_do_nothing()
                .execute(&self.connection)?;
            let new_trade_offers = tx_history.trade_offers
                .iter()
                .map(|(tx_idx, trade_offer)| {
                    models::NewTradeOffer {
                        tx: tx_ids[*tx_idx],
                        output_idx: trade_offer.output_idx,
                        input_idx: trade_offer.input_idx,
                        input_tx: trade_offer.input_tx.to_vec(),
                        price_per_token: rational_to_pg_numeric(trade_offer.price_per_token.clone(),
                                                                PRICE_DIGITS),
                        is_inverted: trade_offer.is_inverted,
                        script_price: trade_offer.script_price,
                        sell_amount_token_base: trade_offer.sell_amount_token_base.into(),
                        receiving_address: trade_offer.receiving_address.bytes().to_vec(),
                    }
                })
                .collect::<Vec<_>>();
            diesel::insert_into(trade_offer::table)
                .values(&new_trade_offers)
                .on_conflict_do_nothing()
                .execute(&self.connection)?;
            Ok(())
        })
    }

    pub fn last_update(&self, subject_type: UpdateSubjectType)
            -> QueryResult<Option<UpdateHistory>> {
        let update: Option<models::UpdateHistory> = update_history::table
            .filter(update_history::subject_type.eq(subject_type as i32))
            .order(update_history::timestamp.desc())
            .limit(1)
            .first::<models::UpdateHistory>(&self.connection)
            .optional()?;
        Ok(update.map(|update| {
            UpdateHistory {
                last_height: update.last_height,
                last_tx_hash: update.last_tx_hash,
                subject_type,
                subject_hash: update.subject_hash,
                completed: update.completed,
            }
        }))
    }

    pub fn add_update_history(&self, update_history: &UpdateHistory) -> QueryResult<()> {
        diesel::insert_into(update_history::table)
            .values(&models::NewUpdateHistory {
                last_height: update_history.last_height,
                last_tx_hash: update_history.last_tx_hash.clone(),
                last_tx_hash_be: update_history.last_tx_hash.as_ref().map(|hash| {
                    let mut hash = hash.clone();
                    hash.reverse();
                    hash
                }),
                subject_type: update_history.subject_type as i32,
                subject_hash: update_history.subject_hash.clone(),
                completed: update_history.completed,
            })
            .execute(&self.connection)?;
        Ok(())
    }

    pub fn add_tokens(&self, tokens: &[Token]) -> QueryResult<()> {
        diesel::insert_into(token::table)
            .values(&tokens.iter()
                .map(|token| {
                    models::NewToken {
                        hash: token.hash.to_vec(),
                        decimals: token.decimals,
                        timestamp: token.timestamp,
                        version_type: token.version_type,
                        document_uri: token.document_uri.clone(),
                        symbol: token.symbol.clone(),
                        name: token.name.clone(),
                        document_hash: token.document_hash.clone(),
                        initial_supply: token.initial_supply.into(),
                        current_supply: token.current_supply.into(),
                        block_created_height: token.block_created_height,
                    }
                })
                .collect::<Vec<_>>()
            )
            .on_conflict(token::hash)
            .do_update().set(token::current_supply.eq(token::current_supply))
            .execute(&self.connection)?;
        Ok(())
    }

    pub fn token(&self, token_hash: &[u8; 32]) -> QueryResult<Option<Token>> {
        let token: Option<models::Token> = token::table
            .filter(token::hash.eq(token_hash.to_vec()))
            .first::<models::Token>(&self.connection)
            .optional()?;
        Ok(token.map(|token| {
            Token {
                hash: {
                    let mut hash = [0; 32];
                    hash.copy_from_slice(&token.hash);
                    hash
                },
                decimals: token.decimals,
                timestamp: token.timestamp,
                version_type: token.version_type,
                document_uri: token.document_uri,
                symbol: token.symbol,
                name: token.name,
                document_hash: token.document_hash,
                initial_supply: SLPAmount::from_numeric_decimals(&token.initial_supply,
                                                                 token.decimals as u32),
                current_supply: SLPAmount::from_numeric_decimals(&token.current_supply,
                                                                 token.decimals as u32),
                block_created_height: token.block_created_height,
            }
        }))
    }

    pub fn update_utxo_set(&self, address: &cashcontracts::Address) -> QueryResult<()> {
        self.connection.transaction(|| {
            diesel::delete(utxo_address::table)
                .filter(
                    utxo_address::address.eq(address.bytes().to_vec())
                )
                .execute(&self.connection)?;
            diesel::insert_into(utxo_address::table)
                .values(
                    tx_output::table
                        .left_join(tx::table)
                        .left_outer_join(tx_input::table.on(
                            tx::hash.eq(tx_input::output_tx)
                                .and(tx_output::idx.eq(tx_input::output_idx))
                        ))
                        .filter(tx_input::tx.is_null())
                        .filter(tx_output::address.eq(address.bytes().to_vec()))
                        .select((tx_output::tx, tx_output::idx, tx_output::address))
                )
                .execute(&self.connection)?;
            Ok(())
        })
    }

    pub fn update_utxo_set_exch(&self) -> QueryResult<()> {
        self.connection.transaction(|| {
            diesel::delete(utxo_trade_offer::table)
                .execute(&self.connection)?;
            diesel::insert_into(utxo_trade_offer::table)
                .values(
                    tx_output::table
                        .left_join(tx::table)
                        .left_join(trade_offer::table.on(
                            tx_output::tx.eq(trade_offer::tx)
                                .and(tx_output::idx.nullable().eq(trade_offer::output_idx))
                        ))
                        .left_outer_join(tx_input::table.on(
                            tx::hash.eq(tx_input::output_tx)
                                .and(tx_output::idx.eq(tx_input::output_idx))
                        ))
                        .filter(tx_input::tx.is_null())
                        .select((tx_output::tx, tx_output::idx))
                )
                .execute(&self.connection)?;
            Ok(())
        })
    }
}
