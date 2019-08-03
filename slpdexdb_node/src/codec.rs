use std::io;
use tokio_io::codec::{Decoder, Encoder};
use bytes::{BytesMut};

use crate::message_packet::MessagePacket;
use crate::message_header::{MessageHeader, HEADER_SIZE};

pub struct MessageCodec;


impl Decoder for MessageCodec {
    type Item = MessagePacket;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let src_len = src.len();
        if src_len < HEADER_SIZE {
            return Ok(None)
        }
        let mut cur = io::Cursor::new(src.as_ref());
        let header = MessageHeader::from_stream(&mut cur)?;
        let msg_size = header.payload_size() as usize + HEADER_SIZE;
        if src_len < msg_size {
            return Ok(None)
        }
        let packet = MessagePacket::from_header_stream(header, &mut cur)?;
        src.advance(msg_size);
        Ok(Some(packet))
    }
}

impl Encoder for MessageCodec {
    type Item = MessagePacket;
    type Error = io::Error;

    fn encode(&mut self, item: MessagePacket, dst: &mut BytesMut) -> Result<(), Self::Error> {
        item.write_to_bytes(dst);
        Ok(())
    }
}