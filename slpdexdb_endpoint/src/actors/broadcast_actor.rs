use actix::prelude::*;
use slpdexdb_base::Error;
use slpdexdb_db::{OutputType, Utxo, SpentUtxo, NewUtxo, TxDelta};
use slpdexdb_base::SLPAmount;
use std::collections::{HashMap, HashSet};
use std::convert::identity;
use std::sync::Arc;
use crate::msg::{NewTransactions, TxEvent, TxBroadcastEvent};

pub struct UpdateDbUtxosActor;

impl Actor for UpdateDbUtxosActor {
    type Context = Context<Self>;
}

impl Handler<NewTransactions> for UpdateDbUtxosActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: NewTransactions, _ctx: &mut Self::Context) -> Self::Result {
        let mut add_utxos = Vec::new();
        let mut remove_utxos = Vec::new();
        for (idx, tx) in msg.tx_history.txs.iter().enumerate() {
            let trade_offer = msg.tx_history.trade_offers.get(&idx);
            for input in tx.inputs.iter() {
                match &input.output {
                    OutputType::Address(address) if msg.relevant_addresses.contains(address) => {},
                    _ if trade_offer.is_some() => {},
                    _ => continue,
                }
                remove_utxos.push(SpentUtxo {
                    tx_hash: input.output_tx.clone(),
                    vout: input.output_idx,
                })
            }
            for (output_idx, output) in tx.outputs.iter().enumerate() {
                match &output.output {
                    OutputType::Address(address) => {
                        add_utxos.push(NewUtxo::Address {
                            tx_hash: tx.hash.clone(),
                            vout: output_idx as i32,
                            address: address.clone(),
                        });
                    },
                    _ if trade_offer.is_some() => {
                        add_utxos.push(NewUtxo::TradeOffer {
                            tx_hash: tx.hash.clone(),
                            vout: output_idx as i32,
                        });
                    },
                    _ => {},
                }
            }
        }
        msg.db.lock().unwrap().remove_utxos(&remove_utxos)?;
        msg.db.lock().unwrap().add_utxos(&add_utxos)?;
        Ok(())
    }
}

pub struct BroadcastAddressUtxosActor {
    event_broadcast: Addr<BroadcastActor>,
}

impl BroadcastAddressUtxosActor {
    pub fn new(event_broadcast: Addr<BroadcastActor>) -> Self {
        BroadcastAddressUtxosActor { event_broadcast }
    }
}

impl Actor for BroadcastAddressUtxosActor {
    type Context = Context<Self>;
}

impl Handler<NewTransactions> for BroadcastAddressUtxosActor {
    type Result = Response<(), Error>;

    fn handle(&mut self, msg: NewTransactions, _ctx: &mut Self::Context) -> Self::Result {
        let mut address_add_utxos = HashMap::new();
        let mut address_remove_utxos = HashMap::new();
        for tx in msg.tx_history.txs.iter() {
            for input in tx.inputs.iter() {
                if let OutputType::Address(address) = &input.output {
                    if !msg.relevant_addresses.contains(address) { continue; }
                    address_remove_utxos
                        .entry(address.clone())
                        .or_insert_with(Vec::new)
                        .push(SpentUtxo {
                            tx_hash: input.output_tx.clone(),
                            vout: input.output_idx,
                        });
                }
            }
            for (output_idx, output) in tx.outputs.iter().enumerate() {
                if let OutputType::Address(ref address) = output.output {
                    if !msg.relevant_addresses.contains(address) { continue; }
                    address_add_utxos
                        .entry(address.clone())
                        .or_insert_with(Vec::new)
                        .push(Utxo {
                            tx_hash: tx.hash.clone(),
                            vout: output_idx as i32,
                            token_hash: tx.tx_type.token_hash().cloned(),
                            value_satoshis: output.value_satoshis,
                            value_token: output.value_token,
                        });
                }
            }
        }
        Response::fut(
            self.event_broadcast
                .send(TxBroadcastEvent::AddressUtxoDelta {
                    add_utxos: address_add_utxos,
                    remove_utxos: address_remove_utxos,
                    subscribers: msg.subscribers.clone(),
                })
                .from_err()
                .and_then(identity)
        )
    }
}

pub struct BroadcastTradeOfferUtxosActor {
    event_broadcast: Addr<BroadcastActor>,
}

impl BroadcastTradeOfferUtxosActor {
    pub fn new(event_broadcast: Addr<BroadcastActor>) -> Self {
        BroadcastTradeOfferUtxosActor { event_broadcast }
    }
}

impl Actor for BroadcastTradeOfferUtxosActor {
    type Context = Context<Self>;
}

impl Handler<NewTransactions> for BroadcastTradeOfferUtxosActor {
    type Result = Response<(), Error>;

    fn handle(&mut self, msg: NewTransactions, _ctx: &mut Self::Context) -> Self::Result {
        let mut token_add_utxos = HashMap::new();
        let mut token_remove_utxos = HashMap::new();
        for (idx, tx) in msg.tx_history.txs.iter().enumerate() {
            let (trade_offer, token_hash) =
                match (msg.tx_history.trade_offers.get(&idx), tx.tx_type.token_hash()) {
                    (Some(trade_offer), Some(token_hash)) => (trade_offer, token_hash.clone()),
                    _ => continue,
                };
            if trade_offer.output_idx.is_some() {
                token_add_utxos
                    .entry(token_hash.clone())
                    .or_insert_with(Vec::new)
                    .push(trade_offer.clone());
            }
            token_remove_utxos
                .entry(token_hash)
                .or_insert_with(Vec::new)
                .push(SpentUtxo {
                    tx_hash: trade_offer.input_tx,
                    vout: trade_offer.input_idx,
                });
        }
        Response::fut(
            self.event_broadcast
                .send(TxBroadcastEvent::TradeOfferUtxoDelta {
                    add_utxos: token_add_utxos,
                    remove_utxos: token_remove_utxos,
                    subscribers: msg.subscribers.clone(),
                })
                .from_err()
                .and_then(identity)
        )
    }
}

pub struct BroadcastTxHistoryActor {
    event_broadcast: Addr<BroadcastActor>,
}

impl BroadcastTxHistoryActor {
    pub fn new(event_broadcast: Addr<BroadcastActor>) -> Self {
        BroadcastTxHistoryActor { event_broadcast }
    }
}

impl Actor for BroadcastTxHistoryActor {
    type Context = Context<Self>;
}

impl Handler<NewTransactions> for BroadcastTxHistoryActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: NewTransactions, _ctx: &mut Self::Context) -> Self::Result {
        let input_txs = msg.db.lock().unwrap().tx_outputs(
            msg.tx_history.txs.iter().flat_map(|tx| {
                tx.inputs.iter().filter_map(|input| {
                    if msg.relevant_addresses.contains(input.output.address()?) {
                        Some(input.output_tx.clone())
                    } else {
                        None
                    }
                })
            })
        )?;
        let mut address_tx_deltas = HashMap::new();
        for tx in msg.tx_history.txs.iter() {
            let decimals = tx.outputs.iter()
                .map(|output| output.value_token.decimals())
                .next()
                .unwrap_or(0);
            let mut tx_deltas = HashMap::new();
            for input in tx.inputs.iter() {
                input.output.address()
                    .filter(|address| msg.relevant_addresses.contains(address))
                    .and_then(|address| {
                        let input_output = input_txs.get(&(input.output_tx, input.output_idx))?;
                        let (a_satoshis, a_token) = (input_output.value_satoshis,
                                                      SLPAmount::from_numeric_decimals(
                                                          &input_output.value_token_base,
                                                          decimals,
                                                      ));
                        tx_deltas
                            .entry(address.clone())
                            .and_modify(|b: &mut (i64, SLPAmount)| {
                                b.0 -= a_satoshis;
                                b.1 -= a_token;
                            })
                            .or_insert((-a_satoshis, -a_token));
                        Some(())
                    });
            }
            for output in tx.outputs.iter() {
                output.output.address()
                    .filter(|address| msg.relevant_addresses.contains(address))
                    .map(|address| {
                        let a_satoshis = output.value_satoshis as i64;
                        let a_token = output.value_token;
                        tx_deltas
                            .entry(address.clone())
                            .and_modify(|b: &mut (i64, SLPAmount)| {
                                b.0 += a_satoshis;
                                b.1 += a_token;
                            })
                            .or_insert((a_satoshis, a_token));
                    });
            }
            for (address, (delta_satoshis, delta_token)) in tx_deltas {
                address_tx_deltas
                    .entry(address)
                    .or_insert_with(Vec::new)
                    .push(TxDelta {
                        tx_hash: tx.hash.clone(),
                        token_hash: tx.tx_type.token_hash().cloned(),
                        timestamp: msg.now,
                        delta_satoshis,
                        delta_token,
                    })
            }
        }
        self.event_broadcast
            .do_send(TxBroadcastEvent::AddressNewTxDeltas {
                tx_deltas: address_tx_deltas,
                subscribers: msg.subscribers.clone(),
            });
        Ok(())
    }
}

pub struct BroadcastActor;

impl Actor for BroadcastActor {
    type Context = Context<Self>;
}

impl Handler<TxBroadcastEvent> for BroadcastActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: TxBroadcastEvent, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            TxBroadcastEvent::AddressUtxoDelta { mut add_utxos, mut remove_utxos, subscribers } => {
                let subscribers = subscribers.lock().unwrap();
                let addresses = add_utxos
                    .keys().chain(remove_utxos.keys()).cloned().collect::<HashSet<_>>();
                for address in addresses {
                    if let Some(subscribers) = subscribers.subscribers_address.get(&address) {
                        let new_msg = TxEvent::AddressUtxoDelta {
                            add_utxos: Arc::new(add_utxos.remove(&address).unwrap_or_default()),
                            remove_utxos: Arc::new(remove_utxos.remove(&address).unwrap_or_default()),
                        };
                        for subscriber in subscribers {
                            subscriber.do_send(new_msg.clone()).unwrap();  // TODO: handle error
                        }
                    }
                }
            },
            TxBroadcastEvent::TradeOfferUtxoDelta { mut add_utxos, mut remove_utxos, subscribers } => {
                let subscribers = subscribers.lock().unwrap();
                let tokens = add_utxos
                    .keys().chain(remove_utxos.keys()).cloned().collect::<HashSet<_>>();
                for token in tokens {
                    if let Some(subscribers) = subscribers.subscribers_token.get(&token) {
                        let add_utxos = Arc::new(add_utxos.remove(&token).unwrap_or_default());
                        let remove_utxos = Arc::new(remove_utxos.remove(&token).unwrap_or_default());
                        for subscriber in subscribers {
                            subscriber.do_send(TxEvent::TradeOfferUtxoDelta {
                                token_hash: token.clone(),
                                add_utxos: add_utxos.clone(),
                                remove_utxos: remove_utxos.clone(),
                            }).unwrap();   // TODO: handle error
                        }
                    }
                }
            },
            TxBroadcastEvent::AddressNewTxDeltas { tx_deltas, subscribers } => {
                let subscribers = subscribers.lock().unwrap();
                for (address, tx_delta) in tx_deltas {
                    if let Some(subscribers) = subscribers.subscribers_address.get(&address) {
                        let new_msg = TxEvent::AddressNewTxDeltas {
                            tx_deltas: Arc::new(tx_delta),
                        };
                        for subscriber in subscribers {
                            subscriber.do_send(new_msg.clone()).unwrap();   // TODO: handle error
                        }
                    }
                }
            },
        }
        Ok(())
    }
}
