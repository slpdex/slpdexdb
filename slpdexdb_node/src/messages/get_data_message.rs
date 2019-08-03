use crate::message_packet::MessagePacket;
use crate::message::NodeMessage;
use crate::messages::InvVector;
use cashcontracts::serialize::write_var_int;
use byteorder::{LittleEndian, WriteBytesExt};
use std::io::{self, Write};


#[derive(Clone, Debug)]
pub struct GetDataMessage {
    pub inv_vectors: Vec<InvVector>,
}

impl NodeMessage for GetDataMessage {
    fn command() -> &'static [u8] { b"getdata" }

    fn packet(&self) -> MessagePacket {
        let mut payload = Vec::new();
        write_var_int(&mut payload, self.inv_vectors.len() as u64).unwrap();
        for inv_vector in self.inv_vectors.iter() {
            payload.write_u32::<LittleEndian>(inv_vector.type_id as u32).unwrap();
            payload.write(&inv_vector.hash).unwrap();
        }
        MessagePacket::from_payload(Self::command(), payload)
    }

    fn from_stream(_stream: &mut impl io::Read) -> io::Result<Self> {
        unimplemented!()
    }
}
