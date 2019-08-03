use crate::message_packet::MessagePacket;
use crate::message::NodeMessage;
use cashcontracts::Tx;
use std::io;

#[derive(Clone, Debug)]
pub struct TxMessage {
    pub tx: Tx,
}

impl NodeMessage for TxMessage {
    fn command() -> &'static [u8] {
        b"tx"
    }

    fn packet(&self) -> MessagePacket {
        let mut payload = Vec::new();
        self.tx.write_to_stream(&mut payload).unwrap();
        MessagePacket::from_payload(Self::command(), payload)
    }

    fn from_stream(stream: &mut impl io::Read) -> io::Result<Self> {
        Ok(TxMessage {tx: Tx::read_from_stream(stream)? })
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
