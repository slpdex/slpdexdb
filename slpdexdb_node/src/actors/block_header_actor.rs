use actix::prelude::*;
use slpdexdb_base::Error;
use crate::messages::{HeadersMessage, GetDataMessage, GetHeadersMessage};
use crate::message::NodeMessage;
use crate::actors::{NodeActor, IncomingMsg, OutgoingMsg};
use crate::db_query::{DbActor, HeaderTipQuery};
use crate::msg::{Subscribe, HandshakeSuccess};

pub struct BlockHeaderActor {
    pub db: Addr<DbActor>,
    pub node: Addr<NodeActor>,
}

impl Actor for BlockHeaderActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.node.do_send(Subscribe::HandshakeSuccess(ctx.address().recipient()));
    }
}

impl Handler<HandshakeSuccess> for BlockHeaderActor {
    type Result = ();

    fn handle(&mut self, msg: HandshakeSuccess, ctx: &mut Self::Context) -> Self::Result {
        let node = self.node.clone();
        Arbiter::spawn(
            self.db.send(HeaderTipQuery).map(|tip| tip.unwrap())  // TODO: handle error
                .and_then(move |tip| {
                    node.send(OutgoingMsg(GetHeadersMessage {
                        version: 70015,
                        block_locator_hashes: vec![tip.header.hash()],
                        hash_stop: [0; 32],
                    }.packet())).map(|_| ())
                })
                .map_err(|_| ())
        )
    }
}

impl Handler<IncomingMsg<HeadersMessage>> for BlockHeaderActor {
    type Result = ();

    fn handle(&mut self, msg: IncomingMsg<HeadersMessage>, ctx: &mut Self::Context) -> Self::Result {

    }
}
