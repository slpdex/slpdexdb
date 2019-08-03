use actix::prelude::*;
use tokio_tcp::TcpStream;
use tokio_io::io::WriteHalf;
use tokio_codec::FramedRead;
use tokio_io::AsyncRead;
use std::io;
use std::sync::Arc;

use crate::codec::MessageCodec;
use crate::message::NodeMessage;
use crate::messages::{VersionMessage, VerackMessage, InvMessage};
use crate::message_packet::MessagePacket;
use crate::actors::{VersionActor, InvActor};
use crate::msg::{Subscribe, HandshakeSuccess};

pub struct IncomingMsg<M: NodeMessage>(pub Arc<M>);

impl<M: NodeMessage> Message for IncomingMsg<M> {
    type Result = ();
}

pub struct OutgoingMsg(pub MessagePacket);

impl Message for OutgoingMsg {
    type Result = ();
}

pub struct NodeActor {
    framed: actix::io::FramedWrite<WriteHalf<TcpStream>, MessageCodec>,

    subscribers_version: Vec<Recipient<IncomingMsg<VersionMessage>>>,
    subscribers_verack: Vec<Recipient<IncomingMsg<VerackMessage>>>,
    subscribers_inv: Vec<Recipient<IncomingMsg<InvMessage>>>,
    subscribers_handshake: Vec<Recipient<HandshakeSuccess>>,
}

impl NodeActor {
    pub fn create_from_tcp(stream: TcpStream) -> Addr<Self> {
        let local_addr = stream.local_addr().unwrap(); // TODO: handle error
        let peer_addr = stream.peer_addr().unwrap(); // TODO: handle error
        let addr = NodeActor::create(|ctx| {
            let (r, w) = stream.split();
            ctx.add_stream(FramedRead::new(r, MessageCodec));
            NodeActor {
                framed: actix::io::FramedWrite::new(
                    w,
                    MessageCodec,
                    ctx,
                ),
                subscribers_handshake: Vec::new(),
                subscribers_inv: Vec::new(),
                subscribers_version: Vec::new(),
                subscribers_verack: Vec::new(),
            }
        });
        InvActor::start(InvActor { node: addr.clone() });
        VersionActor::start(VersionActor { node: addr.clone(), local_addr, peer_addr });
        //BlockHeaderActor::start(BlockHeaderActor);
        addr
    }
}

impl Actor for NodeActor {
    type Context = Context<Self>;
}

impl actix::io::WriteHandler<io::Error> for NodeActor {
    fn error(&mut self, err: io::Error, _ctx: &mut Self::Context) -> Running {
        eprintln!("error: {}", err);
        Running::Continue
    }
}

impl StreamHandler<MessagePacket, io::Error> for NodeActor {
    fn handle(&mut self, msg: MessagePacket, ctx: &mut Context<Self>) {
        let mut cur = io::Cursor::new(msg.payload());
        println!("msg: {}", msg);
        match msg.header().command_name() {
            b"version" => {
                let msg = Arc::new(
                    VersionMessage::from_stream(&mut cur).unwrap()  // TODO: handle error
                );
                for sub in self.subscribers_version.iter() {
                    sub.do_send(IncomingMsg(Arc::clone(&msg)));
                }
            },
            b"inv" => {
                let msg = Arc::new(
                    InvMessage::from_stream(&mut cur).unwrap()  // TODO: handle error
                );
                for sub in self.subscribers_inv.iter() {
                    sub.do_send(IncomingMsg(Arc::clone(&msg)));
                }
            },
            _ => {
            },
        }
    }
}

impl Handler<Subscribe> for NodeActor {
    type Result = ();

    fn handle(&mut self, msg: Subscribe, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            Subscribe::Version(recipient) => self.subscribers_version.push(recipient),
            Subscribe::Verack(recipient) => self.subscribers_verack.push(recipient),
            Subscribe::Inv(recipient) => self.subscribers_inv.push(recipient),
            Subscribe::HandshakeSuccess(recipient) => self.subscribers_handshake.push(recipient),
        }
    }
}

impl Handler<HandshakeSuccess> for NodeActor {
    type Result = ();

    fn handle(&mut self, msg: HandshakeSuccess, ctx: &mut Self::Context) -> Self::Result {
        for sub in self.subscribers_handshake.iter() {
            sub.do_send(msg.clone());
        }
    }
}

impl Handler<OutgoingMsg> for NodeActor {
    type Result = ();

    fn handle(&mut self, msg: OutgoingMsg, _: &mut Self::Context) -> Self::Result {
        self.framed.write(msg.0)
    }
}
