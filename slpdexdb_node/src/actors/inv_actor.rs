use actix::prelude::*;
use crate::messages::{InvMessage, GetDataMessage};
use crate::message::NodeMessage;
use crate::actors::{NodeActor, IncomingMsg, OutgoingMsg};
use crate::msg::Subscribe;

pub struct InvActor {
    pub node: Addr<NodeActor>,
}

impl Actor for InvActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.node.do_send(Subscribe::Inv(ctx.address().recipient()));
    }
}

impl Handler<IncomingMsg<InvMessage>> for InvActor {
    type Result = ();

    fn handle(&mut self, msg: IncomingMsg<InvMessage>, _: &mut Self::Context) -> Self::Result {
        self.node.do_send(OutgoingMsg(GetDataMessage { inv_vectors: msg.0.inv_vectors.clone() }.packet()))
    }
}
