use crate::message_packet::MessagePacket;
use crate::message::NodeMessage;
use slpdexdb_base::BlockHeader;
use std::io::{self, Read};
use cashcontracts::serialize::read_var_int;


pub struct HeadersMessage {
    pub headers: Vec<BlockHeader>,
}

impl NodeMessage for HeadersMessage {
    fn command() -> &'static [u8] {
        b"headers"
    }

    fn packet(&self) -> MessagePacket {
        unimplemented!()
    }

    fn from_stream(stream: &mut impl Read) -> io::Result<HeadersMessage> {
        let n_headers = read_var_int(stream)?;
        let mut headers = Vec::with_capacity(n_headers as usize);
        for _ in 0..n_headers {
            headers.push(BlockHeader::from_stream(stream)?);
            read_var_int(stream)?;
        }
        Ok(HeadersMessage { headers })
    }
}
