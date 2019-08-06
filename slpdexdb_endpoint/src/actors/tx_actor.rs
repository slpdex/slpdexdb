use std::time::{SystemTime, UNIX_EPOCH};
use actix::prelude::*;
use std::collections::HashSet;
use std::convert::identity;
use slpdexdb_base::{Error, SLPDEXConfig};
use slpdexdb_db::{Db, TxSource, TxHistory, OutputType, UpdateSubjectType, UpdateHistory, Utxo, TxDelta};
use slpdexdb_node::actors::{NodeActor, IncomingMsg};
use slpdexdb_node::msg::Subscribe;
use slpdexdb_node::messages::TxMessage;
use crate::msg::{ActivateAddress, DeactivateAddress, ResyncAddress, FetchAddressUtxos, FetchAddressTxDeltas};
use crate::actors::ResyncActor;

pub struct TxActor {
    db: Db,
    config: SLPDEXConfig,
    resync: Addr<ResyncActor>,
}

impl TxActor {
    pub fn new(db: Db,
               config: SLPDEXConfig,
               resync: Addr<ResyncActor>) -> Self {
        TxActor { db, config, resync }
    }
}

impl Actor for TxActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        //self.node.do_send(Subscribe::Tx(ctx.address().recipient()));
    }
}

impl Handler<IncomingMsg<TxMessage>> for TxActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: IncomingMsg<TxMessage>, ctx: &mut Self::Context) -> Self::Result {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let tx = msg.0.tx.clone();
        let mut history = TxHistory::from_txs(&[tx], timestamp, &self.config, &self.db);
        let addresses = history.txs.iter()
            .flat_map(|tx| {
                tx.outputs.iter()
                    .map(|output| output.output.clone())
                    .chain(tx.inputs.iter().map(|input| input.output.clone()))
                    .filter_map(|output| match output {
                        OutputType::Address(address) => Some(address),
                        _ => None,
                    })
            })
            .collect::<Vec<_>>();
        if history.trade_offers.len() == 0 &&
                !self.db.is_active_address(&addresses)?.iter().any(|x| *x) {
            return Ok(())
        }
        history.validate_slp(&TxSource::new(), &self.db, &self.config)?;
        self.db.add_tx_history(&history)?;
        Ok(())
    }
}

impl Handler<ActivateAddress> for TxActor {
    type Result = Response<(), Error>;

    fn handle(&mut self, msg: ActivateAddress, ctx: &mut Self::Context) -> Self::Result {
        let ActivateAddress(address) = msg;
        let resync = self.resync.clone();
        Response::fut(
            futures::future::result(self.db.set_address_active(&address, true)).from_err()
                .and_then(move |_| resync.send(ResyncAddress(address)).from_err())
                .and_then(identity)
        )
    }
}

impl Handler<DeactivateAddress> for TxActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: DeactivateAddress, ctx: &mut Self::Context) -> Self::Result {
        let DeactivateAddress(address) = msg;
        Ok(self.db.set_address_active(&address, false)?)
    }
}

impl Handler<FetchAddressUtxos> for TxActor {
    type Result = Result<Vec<Utxo>, Error>;

    fn handle(&mut self, msg: FetchAddressUtxos, ctx: &mut Self::Context) -> Self::Result {
        let FetchAddressUtxos(address) = msg;
        Ok(self.db.utxos_address(&address)?)
    }
}

impl Handler<FetchAddressTxDeltas> for TxActor {
    type Result = Result<Vec<TxDelta>, Error>;

    fn handle(&mut self, msg: FetchAddressTxDeltas, ctx: &mut Self::Context) -> Self::Result {
        let FetchAddressTxDeltas(address) = msg;
        Ok(self.db.address_tx_deltas(&address)?)
    }
}
