use std::net::{TcpStream, ToSocketAddrs};
use std::io;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::message::Message;
use crate::version_message::{VersionMessage, VerackMessage};
use crate::inv_message::{InvMessage, ObjectType};
use crate::headers_message::{HeadersMessage};
use crate::get_headers_message::GetHeadersMessage;
use crate::db::Db;

use cashcontracts::tx_hash_to_hex;

pub struct Node {
    connections: Vec<TcpStream>,
    db: Db,
}

impl Node {
    pub fn new(db: Db) -> Self { Node { connections: Vec::new(), db } }

    pub fn connect(&mut self, addr: impl ToSocketAddrs) -> io::Result<()> {
        self.connections.push(TcpStream::connect(addr)?);
        Ok(())
    }

    fn send_version(connection: &mut TcpStream) -> io::Result<()> {
        let unix_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

        let version_msg = VersionMessage {
            version: 70015,
            services: 0,
            timestamp: unix_time as i64,
            recv_services: 1,
            recv_addr: connection.peer_addr()?.ip(),
            recv_port: connection.peer_addr()?.port(),
            send_services: 0,
            send_addr: connection.local_addr()?.ip(),
            send_port: connection.local_addr()?.port(),
            nonce: rand::random(),
            user_agent: b"/slpdexdb:0.0.1/".to_vec(),
            start_height: 0,
            relay: true,
        };
        version_msg.message().write_to_stream(connection)?;
        Ok(())
    }

    fn request_get_headers(db: &Db, connection: &mut TcpStream) -> io::Result<()> {
        let hash = match db.header_tip().unwrap() {  // TODO: handle error
            Some(header) => header.hash(),
            None => {
                db.add_header(&crate::block::GENESIS).unwrap();
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

    fn handle_message(db: &Db, connection: &mut TcpStream, msg: &Message) -> io::Result<()> {
        match msg.header().command_name() {
            b"verack" => {
                VerackMessage.message().write_to_stream(connection)?;
                Message::from_payload(b"sendheaders", vec![]).write_to_stream(connection)?;
                Self::request_get_headers(db, connection)?;
            },
            b"inv" => {
                let inv_message = InvMessage::from_payload(msg.payload())?;
                //println!("{}", inv_message);
                for inv_vector in inv_message.inv_vectors.iter() {
                    if inv_vector.type_id == ObjectType::Tx {
                        println!("New tx: {}", tx_hash_to_hex(&inv_vector.hash));
                    }
                }
            },
            b"headers" => {
                let mut cur = io::Cursor::new(msg.payload());
                let headers = HeadersMessage::from_stream(&mut cur)?;
                if headers.headers.len() == 0 {return Ok(());}
                for header in headers.headers {
                    println!("new header: {}", header);
                    db.add_header(&header).unwrap();
                }
                Self::request_get_headers(db, connection)?;
            },
            _ => {},
        }
        Ok(())
    }

    pub fn run_forever(&mut self) -> io::Result<()> {
        let connection = &mut self.connections[0];
        Self::send_version(connection)?;
        loop {
            match Message::from_stream(connection) {
                Ok(msg) => {
                    //println!("msg: {}", msg);
                    Self::handle_message(&self.db, connection, &msg)?;
                },
                Err(err) => {
                    println!("Invalid message: {:?}", err);
                    return Ok(());
                },
            }
        }
    }
}
