use actix::prelude::*;
use cashcontracts::{Address, tx_hash_to_hex, tx_hex_to_hash};
use std::convert::identity;
use actix_web_actors::ws;
use slpdexdb_base::{Error, convert_numeric};
use serde::Deserialize;
use json::{object, JsonValue, stringify};
use std::sync::Arc;
use crate::actors::TxActor;
use crate::msg::{ActivateAddress, FetchAddressUtxos, FetchAddressTxDeltas, SubscribeToEvent,
                 UnsubscribeFromEvent, TxEvent};


#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum WsIncomingMessage {
    ListenToTokens {
        #[serde(rename = "tokenIdsHex")]
        token_ids_hex: Vec<String>,
    }
}

impl Message for WsIncomingMessage {
    type Result = Result<(), Error>;
}

pub struct WsActor {
    address: Address,
    tx: Addr<TxActor>,
}

impl WsActor {
    pub fn new(address: Address, tx: Addr<TxActor>) -> Self {
        WsActor { address, tx }
    }
}

impl Actor for WsActor {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let address = self.address.clone();
        let address2 = self.address.clone();
        let address3 = self.address.clone();
        let tx = self.tx.clone();
        let tx2 = self.tx.clone();
        let tx3 = self.tx.clone();
        let own_address = ctx.address();
        let own_address2 = ctx.address();
        let own_address3 = ctx.address();
        Arbiter::spawn(
            self.tx.send(ActivateAddress(self.address.clone())).from_err().and_then(identity)
                .and_then(move |_| {
                    tx.send(FetchAddressUtxos(address)).from_err().and_then(identity)
                })
                .and_then(move |utxos| own_address.send(
                    TxEvent::AddressUtxoDelta { add_utxos: Arc::new(utxos),
                                                remove_utxos: Arc::new(vec![]) }
                ).from_err())
                .and_then(move |_| {
                    tx2.send(FetchAddressTxDeltas(address2)).from_err().and_then(identity)
                })
                .and_then(move |tx_deltas| own_address2.send(
                    TxEvent::AddressNewTxDeltas { tx_deltas: Arc::new(tx_deltas) }
                ).from_err())
                .and_then(move |_| {
                    tx3.send(SubscribeToEvent::Address(address3, own_address3.recipient()))
                        .from_err()
                })
                .map_err(|err| eprintln!("Error: {}", err))
        )
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for WsActor {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        println!("WS: {:?}", msg);
        match msg {
            ws::Message::Ping(msg) => {
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {}
            ws::Message::Text(text) => {
                match serde_json::from_str::<WsIncomingMessage>(&text) {
                    Ok(msg) => ctx.address().do_send(msg),
                    Err(err) => {
                        eprintln!("ws error: {}", err);
                    },
                }
            },
            ws::Message::Binary(_bin) => {},
            ws::Message::Close(_) => {
                self.tx.do_send(UnsubscribeFromEvent::Address(self.address.clone(),
                                                              ctx.address().recipient()));
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

impl Handler<TxEvent> for WsActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: TxEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            TxEvent::AddressUtxoDelta { add_utxos, remove_utxos } => {
                ctx.text(stringify(
                    object!{
                        "type" => "AddressUtxo",
                        "addUtxos" => JsonValue::Array(
                            add_utxos.iter()
                                .map(|utxo| object!{
                                    "tx" => tx_hash_to_hex(&utxo.tx_hash),
                                    "vout" => utxo.vout,
                                    "valueSatoshis" => utxo.value_satoshis,
                                    "valueToken" => format!("{}", utxo.value_token),
                                    "valueTokenBase" => utxo.value_token.base_amount().to_string(),
                                    "tokenIdHex" => utxo.token_hash.map(|token| tx_hash_to_hex(&token)),
                                })
                                .collect()
                        ),
                        "removeUtxos" => JsonValue::Array(
                            remove_utxos.iter()
                                .map(|utxo| object!{
                                    "tx" => tx_hash_to_hex(&utxo.tx_hash),
                                    "vout" => utxo.vout,
                                })
                                .collect()
                        ),
                    }
                ))
            },
            TxEvent::TradeOfferUtxoDelta { token_hash, add_utxos, remove_utxos } => {
                ctx.text(stringify(
                    object!{
                        "type" => "TradeOfferUtxo",
                        "addUtxos" => JsonValue::Array(
                            add_utxos.iter()
                                .map(|trade_offer| object!{
                                    "tx" => tx_hash_to_hex(&trade_offer.tx),
                                    "outputVout" => trade_offer.output_idx,
                                    "inputTx" => tx_hash_to_hex(&trade_offer.input_tx),
                                    "inputVout" => trade_offer.input_idx,
                                    "pricePerToken" => format!("{}", convert_numeric::PrettyRational(
                                        trade_offer.price_per_token.clone()
                                    )),
                                    "scriptPrice" => trade_offer.script_price.to_string(),
                                    "isInverted" => trade_offer.is_inverted,
                                    "sellAmountTokenBase" => trade_offer.sell_amount_token
                                        .base_amount().to_string(),
                                    "receivingAddress" => trade_offer.receiving_address.cash_addr(),
                                    "tokenIdHex" => tx_hash_to_hex(&token_hash),
                                })
                                .collect()
                        ),
                        "removeUtxos" => JsonValue::Array(
                            remove_utxos.iter()
                                .map(|utxo| object!{
                                    "tx" => tx_hash_to_hex(&utxo.tx_hash),
                                    "vout" => utxo.vout,
                                })
                                .collect()
                        ),
                    }
                ))
            },
            TxEvent::AddressNewTxDeltas { tx_deltas } => {
                ctx.text(stringify(
                    object!{
                        "type" => "TxHistory",
                        "addTxHistory" => JsonValue::Array(
                            tx_deltas.iter()
                                .map(|tx_delta| object!{
                                    "tx" => tx_hash_to_hex(&tx_delta.tx_hash),
                                    "deltaSatoshis" => tx_delta.delta_satoshis,
                                    "deltaToken" => format!("{}", tx_delta.delta_token),
                                    "deltaTokenBase" => tx_delta.delta_token.base_amount().to_string(),
                                    "tokenIdHex" => tx_delta.token_hash.map(|token| tx_hash_to_hex(&token)),
                                    "timestamp" => tx_delta.timestamp,
                                })
                                .collect()
                        ),
                    }
                ))
            },
        }
        Ok(())
    }
}

impl Handler<WsIncomingMessage> for WsActor {
    type Result = Response<(), Error>;

    fn handle(&mut self, msg: WsIncomingMessage, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            WsIncomingMessage::ListenToTokens { token_ids_hex } => {
                println!("subscribe to {:?}", token_ids_hex);
                let token_hashes = token_ids_hex.iter()
                    .filter_map(|token_hash| tx_hex_to_hash(token_hash))
                    .collect();
                Response::fut(
                    self.tx
                        .send(SubscribeToEvent::Tokens(token_hashes, ctx.address().recipient()))
                        .from_err()
                )
            }
        }
    }
}
