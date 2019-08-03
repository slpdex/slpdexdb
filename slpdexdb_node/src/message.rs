use crate::message_packet::MessagePacket;

use std::io;

pub trait NodeMessage: Sized {
    fn command() -> &'static [u8];
    fn packet(&self) -> MessagePacket;
    fn from_stream(stream: &mut impl io::Read) -> io::Result<Self>;
}
