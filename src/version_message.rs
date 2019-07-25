use crate::message::Message;
use cashcontracts::serialize::{read_var_str, write_var_str};
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{io, io::{Write, Read}};
use std::net::{IpAddr};


#[derive(Clone, Debug)]
pub struct VersionMessage {
    pub version: i32,
    pub services: u64,
    pub timestamp: i64,
    pub recv_services: u64,
    pub recv_addr: IpAddr,
    pub recv_port: u16,
    pub send_services: u64,
    pub send_addr: IpAddr,
    pub send_port: u16,
    pub nonce: u64,
    pub user_agent: Vec<u8>,
    pub start_height: i32,
    pub relay: bool,
}

impl VersionMessage {
    pub fn command() -> &'static [u8] {
        b"version"
    }

    fn ip_octets(ip: IpAddr) -> [u8; 16] {
        match ip {
            IpAddr::V4(ip) => ip.to_ipv6_mapped().octets(),
            IpAddr::V6(ip) => ip.octets(),
        }
    }

    pub fn message(&self) -> Message {
        let mut payload = Vec::new();
        payload.write_i32::<LittleEndian>(self.version).unwrap();
        payload.write_u64::<LittleEndian>(self.services).unwrap();
        payload.write_i64::<LittleEndian>(self.timestamp).unwrap();

        payload.write_u64::<LittleEndian>(self.recv_services).unwrap();
        payload.write(&Self::ip_octets(self.recv_addr)).unwrap();
        payload.write_u16::<LittleEndian>(self.recv_port).unwrap();

        payload.write_u64::<LittleEndian>(self.send_services).unwrap();
        payload.write(&Self::ip_octets(self.send_addr)).unwrap();
        payload.write_u16::<LittleEndian>(self.send_port).unwrap();

        payload.write_u64::<LittleEndian>(self.nonce).unwrap();
        write_var_str(&mut payload, &self.user_agent).unwrap();
        payload.write_i32::<LittleEndian>(self.start_height).unwrap();
        payload.write_u8(if self.relay {1} else {0}).unwrap();

        Message::from_payload(Self::command(), payload)
    }

    pub fn from_payload(payload: &[u8]) -> VersionMessage {
        let mut cur = io::Cursor::new(payload);
        let version = cur.read_i32::<LittleEndian>().unwrap();
        let services = cur.read_u64::<LittleEndian>().unwrap();
        let timestamp = cur.read_i64::<LittleEndian>().unwrap();

        let recv_services = cur.read_u64::<LittleEndian>().unwrap();
        let mut recv_addr_bytes = [0; 16];
        cur.read(&mut recv_addr_bytes).unwrap();
        let recv_addr = IpAddr::from(recv_addr_bytes);
        let recv_port = cur.read_u16::<LittleEndian>().unwrap();

        let send_services = cur.read_u64::<LittleEndian>().unwrap();
        let mut send_addr_bytes = [0; 16];
        cur.read(&mut send_addr_bytes).unwrap();
        let send_addr = IpAddr::from(send_addr_bytes);
        let send_port = cur.read_u16::<LittleEndian>().unwrap();

        let nonce = cur.read_u64::<LittleEndian>().unwrap();
        let user_agent = read_var_str(&mut cur).unwrap();
        let start_height = cur.read_i32::<LittleEndian>().unwrap();
        let relay = cur.read_u8().unwrap() > 0;
        VersionMessage {
            version, services, timestamp, recv_services, recv_addr, recv_port, send_services,
            send_addr, send_port, nonce, user_agent, start_height, relay,
        }
    }
}

pub struct VerackMessage;

impl VerackMessage {
    pub fn message(&self) -> Message {
        Message::from_payload(b"verack", vec![])
    }
}
