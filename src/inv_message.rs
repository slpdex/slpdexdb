use crate::message::Message;
use cashcontracts::serialize::{read_var_int, write_var_int};
use cashcontracts::tx_hash_to_hex;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{io, io::{Write, Read}};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObjectType {
    Error = 0,
    Tx = 1,
    Block = 2,
    FilteredBlock = 3,
    CmpctBlock = 4,
}

#[derive(Clone, Debug)]
pub struct InvVector {
    pub type_id: ObjectType,
    pub hash: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct InvMessage {
    pub inv_vectors: Vec<InvVector>,
}

impl InvMessage {
    pub fn command() -> &'static [u8] {
        b"inv"
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

    pub fn from_payload(payload: &[u8]) -> io::Result<InvMessage> {
        let mut cur = io::Cursor::new(payload);
        let n_inv = read_var_int(&mut cur)?;
        let mut inv_vectors = Vec::new();
        for _ in 0..n_inv {
            let type_id = match cur.read_u32::<LittleEndian>()? {
                1 => ObjectType::Tx,
                2 => ObjectType::Block,
                _ => continue,
            };
            let mut hash = [0; 32];
            cur.read_exact(&mut hash)?;
            inv_vectors.push(InvVector { type_id, hash });
        }
        Ok(InvMessage {inv_vectors})
    }
}

impl std::fmt::Display for InvMessage {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> Result<(), std::fmt::Error> {
        writeln!(f, "num of invs: {}", self.inv_vectors.len())?;
        for inv_vector in self.inv_vectors.iter() {
            writeln!(f, "{:?}\t{}", inv_vector.type_id, tx_hash_to_hex(&inv_vector.hash))?;
        }
        Ok(())
    }
}
