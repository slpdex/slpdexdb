use actix::prelude::*;
use std::net::SocketAddr;
use std::convert::identity;

use slpdexdb_base::Error;

use crate::messages::{VersionMessage, VerackMessage};
use crate::message::NodeMessage;
use crate::actors::{NodeActor, IncomingMsg, OutgoingMsg};
use crate::msg::{Subscribe, HandshakeSuccess};

pub struct VersionActor {
    pub node: Addr<NodeActor>,
    pub local_addr: SocketAddr,
    pub peer_addr: SocketAddr,
}

impl Actor for VersionActor {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.node.do_send(Subscribe::Version(ctx.address().recipient()));
        self.node.do_send(Subscribe::Verack(ctx.address().recipient()));
        self.node.do_send(OutgoingMsg(VersionMessage::from_addrs(&self.peer_addr, &self.local_addr).packet()));
    }
}

impl Handler<IncomingMsg<VersionMessage>> for VersionActor {
    type Result = Response<(), Error>;

    fn handle(&mut self, msg: IncomingMsg<VersionMessage>, _: &mut Self::Context) -> Self::Result {
        Response::fut(self.node.send(OutgoingMsg(VerackMessage.packet())).from_err())
    }
}

impl Handler<IncomingMsg<VerackMessage>> for VersionActor {
    type Result = Response<(), Error>;

    fn handle(&mut self, msg: IncomingMsg<VerackMessage>, _: &mut Self::Context) -> Self::Result {
        Response::fut(self.node.send(HandshakeSuccess).from_err().and_then(identity))
    }
}
