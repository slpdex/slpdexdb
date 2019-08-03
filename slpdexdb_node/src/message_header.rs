use std::io;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use bytes::{BufMut, BytesMut};

use crate::message_error::MessageError;


#[derive(Clone, Debug)]
pub struct MessageHeader {
    command: [u8; 12],
    payload_size: u32,
    checksum: [u8; 4],
}

pub const MESSAGE_MAGIC: &[u8] = b"\xe3\xe1\xf3\xe8";
pub const HEADER_SIZE: usize = 4 + 12 + 4 + 4;

impl MessageHeader {
    pub fn new(command: [u8; 12],
               payload_size: u32,
               checksum: [u8; 4]) -> Self {
        MessageHeader {
            command, payload_size, checksum,
        }
    }

    pub fn from_stream<R: io::Read>(read: &mut R) -> Result<Self, MessageError> {
        let mut magic = [0; 4];
        let mut command = [0; 12];
        let mut checksum = [0; 4];
        read.read_exact(&mut magic)?;
        if &magic[..] != MESSAGE_MAGIC {
            return Err(MessageError::WrongMagic)
        }
        read.read_exact(&mut command)?;
        let payload_size = read.read_u32::<LittleEndian>()?;
        read.read_exact(&mut checksum)?;
        Ok(MessageHeader {
            command,
            payload_size,
            checksum,
        })
    }

    pub fn write_to_stream<W: io::Write>(&self, write: &mut W) -> io::Result<()> {
        write.write(MESSAGE_MAGIC)?;
        write.write(&self.command)?;
        write.write_u32::<LittleEndian>(self.payload_size)?;
        write.write(&self.checksum)?;
        Ok(())
    }

    pub fn write_to_bytes(&self, bytes: &mut BytesMut) {
        bytes.put(MESSAGE_MAGIC);
        bytes.put(self.command.as_ref());
        bytes.put_u32_le(self.payload_size);
        bytes.put(self.checksum.as_ref());
    }

    pub fn command(&self) -> &[u8; 12] {
        &self.command
    }

    pub fn payload_size(&self) -> u32 {
        self.payload_size
    }

    pub fn checksum(&self) -> &[u8; 4] {
        &self.checksum
    }

    pub fn command_name(&self) -> &[u8] {
        let len = self.command.iter()
            .position(|b| *b == 0)
            .unwrap_or(self.command.len());
        &self.command[..len]
    }
}

impl std::fmt::Display for MessageHeader {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> Result<(), std::fmt::Error> {
        writeln!(f, "command: {}", String::from_utf8_lossy(&self.command))?;
        writeln!(f, "payload size: {}", self.payload_size)?;
        writeln!(f, "checksum: {}", hex::encode(&self.checksum))?;
        Ok(())
    }
}
