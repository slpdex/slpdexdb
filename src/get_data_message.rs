use crate::message::Message;
use crate::inv_message::InvVector;
use cashcontracts::serialize::write_var_int;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::Write;

#[derive(Clone, Debug)]
pub struct GetDataMessage {
    pub inv_vectors: Vec<InvVector>,
}

impl GetDataMessage {
    pub fn command() -> &'static [u8] {
        b"getdata"
    }

    pub fn message(&self) -> Message {
        let mut payload = Vec::new();
        write_var_int(&mut payload, self.inv_vectors.len() as u64).unwrap();
        for inv_vector in self.inv_vectors.iter() {
            payload.write_u32::<LittleEndian>(inv_vector.type_id as u32).unwrap();
            payload.write(&inv_vector.hash).unwrap();
        }
        Message::from_payload(Self::command(), payload)
    }
}
