use std::net::{TcpStream, ToSocketAddrs};
use std::io;

use crate::message_packet::MessagePacket;
use crate::messages::{
    VersionMessage, VerackMessage, InvMessage, ObjectType, HeadersMessage, GetHeadersMessage,
    GetDataMessage, TxMessage,
};
use crate::message::Message;
use crate::tx_history::TxHistory;
use crate::tx_source::TxSource;
use crate::db::Db;
use crate::config::SLPDEXConfig;
use crate::adapter::NodeAdapter;
use slpdexdb_base::Result;


use cashcontracts::tx_hash_to_hex;

pub struct Node<A: NodeAdapter> {
    connections: Vec<TcpStream>,
    adapter: A,
    config: SLPDEXConfig,
}

impl<A: NodeAdapter> Node<A> {
    pub fn new(adapter: A) -> Self {
        Node {
            connections: Vec::new(),
            adapter,
            config: SLPDEXConfig::default(),
        }
    }

    pub fn connect(&mut self, addr: impl ToSocketAddrs) -> io::Result<()> {
        self.connections.push(TcpStream::connect(addr)?);
        Ok(())
    }

    pub fn send_msg(&mut self, msg: &impl Message) {
        let mut disconnected_indices = Vec::new();
        for (idx, connection) in self.connections.iter_mut().enumerate() {
            if Err(err) = msg.packet().write_to_stream(connection) {
                eprintln!("{}", err);
                disconnected_indices.push(idx);
            }
        }
        for idx in disconnected_indices.into_iter().rev() {
            self.connections.remove(idx);
        }
    }

    fn request_get_headers(db: &Db, connection: &mut TcpStream) -> Result<()> {
        let hash = match db.header_tip()? {
            Some((header, _)) => header.hash(),
            None => {
                db.add_headers(&[crate::block::GENESIS.clone()]).unwrap();
                crate::block::GENESIS.hash()
            },
        };
        GetHeadersMessage {
            version: 70015,
            block_locator_hashes: vec![hash],
            hash_stop: [0; 32],
        }.message().write_to_stream(connection)?;
        Ok(())
    }

    fn handle_message(connection: &mut TcpStream, msg: &Message) -> Result<()> {
        match msg.header().command_name() {
            b"verack" => {
                VerackMessage.message().write_to_stream(connection)?;
                Message::from_payload(b"sendheaders", vec![]).write_to_stream(connection)?;
                Self::request_get_headers(db, connection)?;
            },
            b"inv" => {
                let inv_message = InvMessage::from_payload(msg.payload())?;
                //println!("{}", inv_message);
                let mut tx_hashes = Vec::new();
                for inv_vector in inv_message.inv_vectors.iter() {
                    if inv_vector.type_id == ObjectType::Tx {
                        println!("New tx: {}", tx_hash_to_hex(&inv_vector.hash));
                        tx_hashes.push(inv_vector.clone());
                    }
                }
                GetDataMessage {inv_vectors: tx_hashes}.message().write_to_stream(connection)?;
            },
            b"headers" => {
                let mut cur = io::Cursor::new(msg.payload());
                let headers = HeadersMessage::from_stream(&mut cur)?;
                if headers.headers.len() == 0 {return Ok(());}
                for header in headers.headers.iter() {
                    println!("new header: {}", header);
                }
                db.add_headers(&headers.headers)?;
                Self::request_get_headers(db, connection)?;
            },
            b"tx" => {
                let unix_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
                let tx_msg = TxMessage::from_payload(msg.payload())?;
                println!("tx={}", cashcontracts::tx_hash_to_hex(&tx_msg.tx.hash()));
                let mut tx_history = TxHistory::from_txs(&[tx_msg.tx], unix_time as i64, config, db);
                tx_history.txs.iter().for_each(|tx| {
                    match &tx.tx_type {
                        crate::tx_history::TxType::SLP {token_hash, ..} => println!("SLP token={}", hex::encode(token_hash)),
                        _ => {},
                    };
                });
                tx_history.validate_slp(&TxSource::new(), db, config)?;
                tx_history.txs.iter().for_each(|tx| {
                    match &tx.tx_type {
                        crate::tx_history::TxType::SLP {token_hash, ..} => println!("valid SLP token={}", hex::encode(token_hash)),
                        _ => {},
                    };
                });
                tx_history.trade_offers.iter().for_each(|(idx, offer)| {
                    println!("trade offer {}: {:?}", idx, offer);
                });
            },
            _ => {},
        }
        Ok(())
    }

    pub fn run_forever(&mut self) -> Result<()> {
        let connection = &mut self.connections[0];
        Self::send_version(connection)?;
        loop {
            let connection = &mut self.connections[0];
            match Message::from_stream(connection) {
                Ok(msg) => {
                    self.handle_message(connection, &msg)?;
                },
                Err(err) => {
                    println!("Invalid message: {:?}", err);
                    return Ok(());
                },
            }
        }
    }
}
