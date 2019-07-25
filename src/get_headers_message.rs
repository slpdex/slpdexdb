use byteorder::{LittleEndian, WriteBytesExt};
use std::{io::{Write}};
use cashcontracts::serialize::write_var_int;
use crate::message::Message;

#[derive(Clone, Debug)]
pub struct GetHeadersMessage {
    pub version: u32,
    pub block_locator_hashes: Vec<[u8; 32]>,
    pub hash_stop: [u8; 32],
}

impl GetHeadersMessage {
    pub fn command() -> &'static [u8] {
        b"getheaders"
    }

    pub fn message(&self) -> Message {
        let mut payload = Vec::new();
        payload.write_u32::<LittleEndian>(self.version).unwrap();
        write_var_int(&mut payload, self.block_locator_hashes.len() as u64).unwrap();
        for block_locator_hash in self.block_locator_hashes.iter() {
            payload.write(block_locator_hash).unwrap();
        }
        payload.write(&self.hash_stop).unwrap();
        Message::from_payload(Self::command(), payload)
    }
}
