use std::io::{self, Write};
use crate::message_header::{MessageHeader, HEADER_SIZE};
use crate::message_error::MessageError;
use cashcontracts::double_sha256;
use bytes::{BufMut, BytesMut};


#[derive(Clone, Debug)]
pub struct MessagePacket {
    header: MessageHeader,
    payload: Vec<u8>,
}

impl MessagePacket {
    fn _check_checksum(payload: &[u8], checksum: &[u8; 4]) -> Result<(), MessageError> {
        let hash = double_sha256(&payload);
        if &hash[..4] != checksum {
            return Err(MessageError::InvalidChecksum)
        }
        Ok(())
    }

    pub fn from_stream<R: io::Read>(read: &mut R) -> Result<Self, MessageError> {
        let header = MessageHeader::from_stream(read)?;
        let mut payload = vec![0; header.payload_size() as usize];
        read.read_exact(&mut payload[..])?;
        Self::_check_checksum(&payload, header.checksum())?;
        Ok(MessagePacket {
            header,
            payload,
        })
    }

    pub fn from_header_stream<R: io::Read>(header: MessageHeader, read: &mut R)
            -> Result<Self, MessageError> {
        let mut payload = vec![0; header.payload_size() as usize];
        read.read_exact(&mut payload[..])?;
        Self::_check_checksum(&payload, header.checksum())?;
        Ok(MessagePacket { header, payload })
    }

    pub fn from_payload(command: &[u8], payload: Vec<u8>) -> MessagePacket {
        let hash = double_sha256(&payload);
        let mut checksum = [0; 4];
        checksum.copy_from_slice(&hash[..4]);
        let mut command_padded = [0u8; 12];
        io::Cursor::new(&mut command_padded[..]).write(command).unwrap();
        let header = MessageHeader::new(
            command_padded,
            payload.len() as u32,
            checksum,
        );
        MessagePacket {
            header,
            payload,
        }
    }

    pub fn write_to_stream<W: io::Write>(&self, write: &mut W) -> io::Result<()> {
        self.header.write_to_stream(write)?;
        write.write(&self.payload)?;
        Ok(())
    }

    pub fn write_to_bytes(&self, bytes: &mut BytesMut) {
        bytes.reserve(HEADER_SIZE + self.payload.len());
        self.header.write_to_bytes(bytes);
        bytes.put(&self.payload);
    }

    pub fn header(&self) -> &MessageHeader {
        &self.header
    }

    pub fn payload(&self) -> &[u8] {
        &self.payload
    }
}

impl std::fmt::Display for MessagePacket {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.header)?;
        writeln!(f, "payload: {}", hex::encode(&self.payload))?;
        Ok(())
    }
}
