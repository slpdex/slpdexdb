use crate::message::Message;
use cashcontracts::Tx;
use cashcontracts::serialize::{read_var_int, write_var_int};
use byteorder::{LittleEndian, WriteBytesExt};
use std::{io, io::Write};

#[derive(Clone, Debug)]
pub struct TxMessage {
    pub tx: Tx,
}

impl TxMessage {
    pub fn command() -> &'static [u8] {
        b"tx"
    }

    pub fn message(&self) -> Message {
        let mut payload = Vec::new();
        self.tx.write_to_stream(&mut payload).unwrap();
        Message::from_payload(Self::command(), payload)
    }

    pub fn from_payload(payload: &[u8]) -> io::Result<Self> {
        let mut cur = io::Cursor::new(payload);
        Ok(TxMessage {tx: Tx::read_from_stream(&mut cur)? })
    }
}

/*impl std::fmt::Display for TxMessage {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> Result<(), std::fmt::Error> {
        writeln!(f, "num of invs: {}", self.inv_vectors.len())?;
        for inv_vector in self.inv_vectors.iter() {
            writeln!(f, "{:?}\t{}", inv_vector.type_id, tx_hash_to_hex(&inv_vector.hash))?;
        }
        Ok(())
    }
}*/
