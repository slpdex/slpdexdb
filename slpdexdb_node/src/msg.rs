use actix::prelude::*;
use crate::actors::{NodeActor, IncomingMsg};
use crate::messages::{VersionMessage, VerackMessage, InvMessage};

pub enum Subscribe {
    HandshakeSuccess(Recipient<HandshakeSuccess>),
    Version(Recipient<IncomingMsg<VersionMessage>>),
    Verack(Recipient<IncomingMsg<VerackMessage>>),
    Inv(Recipient<IncomingMsg<InvMessage>>),
}

impl Message for Subscribe {
    type Result = ();
}

#[derive(Clone)]
pub struct HandshakeSuccess;

impl Message for HandshakeSuccess {
    type Result = ();
}