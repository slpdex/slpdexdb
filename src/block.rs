use hex_literal::hex;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::{io, io::{Write, Read}};
use cashcontracts::{double_sha256, tx_hash_to_hex};

#[derive(Clone, Debug)]
pub struct BlockHeader {
    pub version: i32,
    pub prev_block: [u8; 32],
    pub merkle_root: [u8; 32],
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
}

pub const GENESIS: BlockHeader = BlockHeader {
    version: 1,
    prev_block: [0; 32],
    merkle_root: hex!("3ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a"),
    timestamp: 1231006505,
    bits: 0x1d00ffff,
    nonce: 2083236893,
};

impl BlockHeader {
    pub fn from_stream(stream: &mut impl Read) -> io::Result<BlockHeader> {
        let version = stream.read_i32::<LittleEndian>()?;
        let mut prev_block = [0; 32];
        stream.read_exact(&mut prev_block)?;
        let mut merkle_root = [0; 32];
        stream.read_exact(&mut merkle_root)?;
        let timestamp = stream.read_u32::<LittleEndian>()?;
        let bits = stream.read_u32::<LittleEndian>()?;
        let nonce = stream.read_u32::<LittleEndian>()?;
        Ok(BlockHeader {
            version, prev_block, merkle_root, timestamp, bits, nonce,
        })
    }

    pub fn write_to_stream(&self, stream: &mut impl Write) -> io::Result<()> {
        stream.write_i32::<LittleEndian>(self.version)?;
        stream.write(&self.prev_block)?;
        stream.write(&self.merkle_root)?;
        stream.write_u32::<LittleEndian>(self.timestamp)?;
        stream.write_u32::<LittleEndian>(self.bits)?;
        stream.write_u32::<LittleEndian>(self.nonce)?;
        Ok(())
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut ser = Vec::with_capacity(80);
        self.write_to_stream(&mut ser).unwrap();
        double_sha256(&ser)
    }
}

impl std::fmt::Display for BlockHeader {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> Result<(), std::fmt::Error> {
        writeln!(f, "BlockHeader: {}", tx_hash_to_hex(&self.hash()))?;
        writeln!(f, " version:     {}", self.version)?;
        writeln!(f, " prev_block:  {}", tx_hash_to_hex(&self.prev_block))?;
        writeln!(f, " merkle_root: {}", tx_hash_to_hex(&self.merkle_root))?;
        writeln!(f, " timestamp:   {}", self.timestamp)?;
        writeln!(f, " bits:        {:x}", self.bits)?;
        writeln!(f, " nonce:       {}", self.nonce)?;
        Ok(())
    }
}
