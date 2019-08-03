use actix::prelude::*;
use slpdexdb_base::Error;
use crate::messages::{HeadersMessage, GetDataMessage, GetHeadersMessage};
use crate::message::NodeMessage;
use crate::message_packet::MessagePacket;
use crate::actors::{NodeActor, IncomingMsg, OutgoingMsg};
use crate::db_query::{DbActor, HeaderTipQuery, AddHeadersQuery};
use crate::msg::{Subscribe, HandshakeSuccess};

pub struct BlockHeaderActor {
    pub db: Addr<DbActor>,
    pub node: Addr<NodeActor>,
}

impl Actor for BlockHeaderActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.node.do_send(Subscribe::HandshakeSuccess(ctx.address().recipient()));
        self.node.do_send(Subscribe::Headers(ctx.address().recipient()));
    }
}

impl BlockHeaderActor {
    fn _fetch_headers(db: Addr<DbActor>, node: Addr<NodeActor>)
            -> impl futures::Future<Item=(), Error=()> {
        db.send(HeaderTipQuery).map(|tip| tip.unwrap())  // TODO: handle error
            .and_then(move |tip| {
                node.send(OutgoingMsg(GetHeadersMessage {
                    version: 70015,
                    block_locator_hashes: vec![tip.header.hash()],
                    hash_stop: [0; 32],
                }.packet())).map(|_| ())
            })
            .map_err(|_| ())
    }
}

impl Handler<HandshakeSuccess> for BlockHeaderActor {
    type Result = ();

    fn handle(&mut self, msg: HandshakeSuccess, ctx: &mut Self::Context) -> Self::Result {
        self.node.do_send(OutgoingMsg(MessagePacket::from_payload(b"sendheaders", vec![])));
        Arbiter::spawn(Self::_fetch_headers(self.db.clone(), self.node.clone()))
    }
}

impl Handler<IncomingMsg<HeadersMessage>> for BlockHeaderActor {
    type Result = ();

    fn handle(&mut self, msg: IncomingMsg<HeadersMessage>, ctx: &mut Self::Context) -> Self::Result {
        let node = self.node.clone();
        let db = self.db.clone();
        if msg.0.headers.len() == 0 {
            return ();
        }
        Arbiter::spawn(
            self.db.send(AddHeadersQuery(msg.0.headers.clone()))
                //.map(|tip| tip.unwrap())  // TODO: handle error
                .map_err(|_| ())
                .and_then(move |x| {
                    Self::_fetch_headers(db, node)
                })
                .map_err(|_| ())
        )
    }
}
