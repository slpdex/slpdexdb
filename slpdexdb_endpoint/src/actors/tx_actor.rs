use std::time::{SystemTime, UNIX_EPOCH};
use actix::prelude::*;
use std::collections::HashSet;
use slpdexdb_base::{Error, SLPDEXConfig};
use slpdexdb_db::{Db, TxSource, TxHistory, OutputType, UpdateSubjectType, UpdateHistory};
use slpdexdb_node::actors::{NodeActor, IncomingMsg};
use slpdexdb_node::msg::Subscribe;
use slpdexdb_node::messages::TxMessage;
use crate::msg::{SetAddressActive, ResyncAddress};
use crate::actors::ResyncActor;

pub struct TxActor {
    db: Db,
    config: SLPDEXConfig,
    node: Addr<NodeActor>,
    resync: Addr<ResyncActor>,
}

impl TxActor {
    pub fn new(db: Db,
               config: SLPDEXConfig,
               node: Addr<NodeActor>,
               resync: Addr<ResyncActor>) -> Self {
        TxActor { db, node, config, resync }
    }
}

impl Actor for TxActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.node.do_send(Subscribe::Tx(ctx.address().recipient()));
    }
}

impl Handler<IncomingMsg<TxMessage>> for TxActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: IncomingMsg<TxMessage>, ctx: &mut Self::Context) -> Self::Result {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let tx = msg.0.tx.clone();
        let history = TxHistory::from_txs(&[tx], timestamp, &self.config, &self.db);
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
        if !self.db.is_active_address(&addresses)?.iter().any(|x| *x) {
            return Ok(())
        }
        self.db.add_tx_history(&history)?;
        Ok(())
    }
}

impl Handler<SetAddressActive> for TxActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: SetAddressActive, ctx: &mut Self::Context) -> Self::Result {
        let address = msg.0;
        let is_active = msg.1;
        self.db.set_address_active(&address, is_active)?;
        if is_active {
            self.resync.do_send(ResyncAddress(address))
        }
        Ok(())
    }
}
