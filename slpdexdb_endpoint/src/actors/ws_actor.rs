use actix::prelude::*;
use cashcontracts::{Address, tx_hash_to_hex};
use std::convert::identity;
use actix_web_actors::ws;
use slpdexdb_db::{Utxo, TxDelta};
use json::{object, array, JsonValue, stringify};
use crate::actors::TxActor;
use crate::msg::{ActivateAddress, FetchAddressUtxos, FetchAddressTxDeltas};


pub struct WsActor {
    address: Address,
    tx: Addr<TxActor>,
}

pub struct GotUtxos(Vec<Utxo>);

impl Message for GotUtxos {
    type Result = ();
}

pub struct GotTxDeltas(Vec<TxDelta>);

impl Message for GotTxDeltas {
    type Result = ();
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
        let tx = self.tx.clone();
        let tx2 = self.tx.clone();
        let own_address = ctx.address();
        let own_address2 = ctx.address();
        Arbiter::spawn(
            self.tx.send(ActivateAddress(self.address.clone())).from_err().and_then(identity)
                .and_then(move |_| {
                    tx.send(FetchAddressUtxos(address)).from_err().and_then(identity)
                })
                .and_then(move |utxos| own_address.send(GotUtxos(utxos)).from_err())
                .and_then(move |_| {
                    tx2.send(FetchAddressTxDeltas(address2)).from_err().and_then(identity)
                })
                .and_then(move |tx_deltas| own_address2.send(GotTxDeltas(tx_deltas)).from_err())
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
            ws::Message::Text(text) => {},
            ws::Message::Binary(bin) => {},
            ws::Message::Close(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

impl Handler<GotUtxos> for WsActor {
    type Result = ();

    fn handle(&mut self, msg: GotUtxos, ctx: &mut Self::Context) -> Self::Result {
        let GotUtxos(utxos) = msg;
        ctx.text(stringify(
            object!{
                "type" => "utxo",
                "addUtxos" => JsonValue::Array(
                    utxos.into_iter()
                        .map(|utxo| {
                            object!{
                                "tx" => tx_hash_to_hex(&utxo.tx_hash),
                                "vout" => utxo.vout,
                                "valueSatoshis" => utxo.value_satoshis,
                                "valueToken" => format!("{}", utxo.value_token),
                                "valueTokenBase" => utxo.value_token.base_amount().to_string(),
                                "tokenIdHex" => utxo.token_hash.map(|token| hex::encode(token)),
                            }
                        })
                        .collect()
                ),
                "deleteUtxos" => array![],
            }
        ));
    }
}

impl Handler<GotTxDeltas> for WsActor {
    type Result = ();

    fn handle(&mut self, msg: GotTxDeltas, ctx: &mut Self::Context) -> Self::Result {
        let GotTxDeltas(tx_deltas) = msg;
        ctx.text(stringify(
            object!{
                "type" => "txHistory",
                "addTxHistory" => JsonValue::Array(
                    tx_deltas.into_iter()
                        .map(|tx_delta| {
                            object!{
                                "tx" => tx_hash_to_hex(&tx_delta.tx_hash),
                                "deltaSatoshis" => tx_delta.delta_satoshis,
                                "deltaToken" => format!("{}", tx_delta.delta_token),
                                "deltaTokenBase" => tx_delta.delta_token.base_amount().to_string(),
                                "tokenIdHex" => tx_delta.token_hash.map(|token| hex::encode(token)),
                                "timestamp" => tx_delta.timestamp,
                            }
                        })
                        .collect()
                ),
            }
        ))
    }
}
