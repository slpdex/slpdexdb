use crate::block::BlockHeader;
use std::{io, io::{Read}};
use cashcontracts::serialize::read_var_int;


pub struct HeadersMessage {
    pub headers: Vec<BlockHeader>,
}

impl HeadersMessage {
    pub fn command() -> &'static [u8] {
        b"headers"
    }

    pub fn from_stream(stream: &mut impl Read) -> io::Result<HeadersMessage> {
        let n_headers = read_var_int(stream)?;
        let mut headers = Vec::with_capacity(n_headers as usize);
        for _ in 0..n_headers {
            headers.push(BlockHeader::from_stream(stream)?);
            read_var_int(stream)?;
        }
        Ok(HeadersMessage { headers })
    }
}
