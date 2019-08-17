use actix::prelude::*;
use std::collections::{HashSet, HashMap};
use std::convert::identity;
use slpdexdb_base::{Error, SLPDEXConfig};
use slpdexdb_db::{Db, Utxo, TxDelta, TradeOffer};
use slpdexdb_node::actors::IncomingMsg;
use slpdexdb_node::messages::TxMessage;
use crate::msg::{ActivateAddress, DeactivateAddress, ResyncAddress, FetchAddressUtxos,
                 FetchAddressTxDeltas, FetchTradeOfferUtxos, SubscribeToEvent, UnsubscribeFromEvent,
                 TxEvent, NewTransactions, ProcessTransactions};
use crate::actors::ResyncActor;
use crate::actors::broadcast_actor::{UpdateDbUtxosActor, BroadcastAddressUtxosActor,
                                     BroadcastTradeOfferUtxosActor, BroadcastTxHistoryActor,
                                     BroadcastActor};

use cashcontracts::Address;
use std::sync::{Mutex, Arc};

pub struct TxSubscribers {
    pub subscribers_address: HashMap<Address, HashSet<Recipient<TxEvent>>>,
    pub subscribers_token: HashMap<[u8; 32], HashSet<Recipient<TxEvent>>>,
}

pub struct TxActor {
    db: Arc<Mutex<Db>>,
    config: SLPDEXConfig,
    resync: Addr<ResyncActor>,
    subscribers: Arc<Mutex<TxSubscribers>>,
    broadcasts: Vec<Recipient<NewTransactions>>,
}

impl TxActor {
    pub fn start_with(db: Arc<Mutex<Db>>,
                      config: SLPDEXConfig,
                      resync: Addr<ResyncActor>) -> Addr<Self> {
        let broadcast = BroadcastActor::start(BroadcastActor);
        let broadcasts = vec![
            UpdateDbUtxosActor::start(UpdateDbUtxosActor).recipient(),
            BroadcastAddressUtxosActor::start(BroadcastAddressUtxosActor::new(broadcast.clone())).recipient(),
            BroadcastTradeOfferUtxosActor::start(BroadcastTradeOfferUtxosActor::new(broadcast.clone())).recipient(),
            BroadcastTxHistoryActor::start(BroadcastTxHistoryActor::new(broadcast.clone())).recipient(),
        ];
        Self::start(TxActor {
            db, config, resync,
            subscribers: Arc::new(Mutex::new(TxSubscribers {
                subscribers_address: HashMap::new(),
                subscribers_token: HashMap::new(),
            })),
            broadcasts,
        })
    }
}

impl Actor for TxActor {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        //self.node.do_send(Subscribe::Tx(ctx.address().recipient()));
    }
}

impl Handler<IncomingMsg<TxMessage>> for TxActor {
    type Result = Response<(), Error>;

    fn handle(&mut self, msg: IncomingMsg<TxMessage>, _ctx: &mut Self::Context) -> Self::Result {
        let tx = msg.0.tx.clone();
        Response::fut(
            self.resync
                .send(ProcessTransactions {
                    db: self.db.clone(),
                    subscribers: self.subscribers.clone(),
                    txs: vec![tx],
                    config: self.config.clone(),
                    broadcasts: self.broadcasts.clone(),
                })
                .from_err()
                .and_then(identity)
        )
    }
}

impl Handler<ActivateAddress> for TxActor {
    type Result = Response<(), Error>;

    fn handle(&mut self, msg: ActivateAddress, _ctx: &mut Self::Context) -> Self::Result {
        let ActivateAddress(address) = msg;
        let resync = self.resync.clone();
        Response::fut(
            futures::future::result(self.db.lock().unwrap().set_address_active(&address, true)).from_err()
                .and_then(move |_| resync.send(ResyncAddress(address)).from_err())
                .and_then(identity)
        )
    }
}

impl Handler<DeactivateAddress> for TxActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: DeactivateAddress, _ctx: &mut Self::Context) -> Self::Result {
        let DeactivateAddress(address) = msg;
        Ok(self.db.lock().unwrap().set_address_active(&address, false)?)
    }
}

impl Handler<FetchAddressUtxos> for TxActor {
    type Result = Result<Vec<Utxo>, Error>;

    fn handle(&mut self, msg: FetchAddressUtxos, _ctx: &mut Self::Context) -> Self::Result {
        let FetchAddressUtxos(address) = msg;
        Ok(self.db.lock().unwrap().utxos_address(&address)?)
    }
}

impl Handler<FetchAddressTxDeltas> for TxActor {
    type Result = Result<Vec<TxDelta>, Error>;

    fn handle(&mut self, msg: FetchAddressTxDeltas, _ctx: &mut Self::Context) -> Self::Result {
        let FetchAddressTxDeltas(address) = msg;
        Ok(self.db.lock().unwrap().address_tx_deltas(&address)?)
    }
}

impl Handler<SubscribeToEvent> for TxActor {
    type Result = ();

    fn handle(&mut self, msg: SubscribeToEvent, _ctx: &mut Self::Context) -> Self::Result {
        let mut subscribers = self.subscribers.lock().unwrap();
        match msg {
            SubscribeToEvent::Address(address, recipient) => {
                subscribers.subscribers_address
                    .entry(address)
                    .or_insert_with(HashSet::new)
                    .insert(recipient);
            },
            SubscribeToEvent::Tokens(token_hashes, recipient) => {
                for (_, subs) in subscribers.subscribers_token.iter_mut() {
                    subs.remove(&recipient);
                }
                for token_hash in token_hashes {
                    subscribers.subscribers_token
                        .entry(token_hash)
                        .or_insert_with(HashSet::new)
                        .insert(recipient.clone());
                }
            },
        };
    }
}

impl Handler<UnsubscribeFromEvent> for TxActor {
    type Result = ();

    fn handle(&mut self, msg: UnsubscribeFromEvent, _ctx: &mut Self::Context) -> Self::Result {
        let mut subscribers = self.subscribers.lock().unwrap();
        match &msg {
            UnsubscribeFromEvent::Address(address, recipient) => {
                subscribers.subscribers_address.get_mut(address).map(|subs| subs.remove(recipient));
            },
        }
    }
}

impl Handler<FetchTradeOfferUtxos> for TxActor {
    type Result = Result<Vec<TradeOffer>, Error>;

    fn handle(&mut self, msg: FetchTradeOfferUtxos, _ctx: &mut Self::Context) -> Self::Result {
        let FetchTradeOfferUtxos(filter) = msg;
        Ok(self.db.lock().unwrap().trade_offer_utxos(filter)?)
    }
}
