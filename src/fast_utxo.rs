use cashcontracts::{TxOutpoint, tx_hash_to_hex};
use byteorder::{LittleEndian, ReadBytesExt};
use std::fs::File;
use std::{io, io::Read};

#[derive(Clone, Debug)]
pub struct Utxo {
    outpoint: TxOutpoint,
    amount: u64,
    script: Vec<u8>,
    block_height: i32,
    flags: u8,
}

pub struct FastUtxoSet {
    file: File,
}

impl FastUtxoSet {
    pub fn new(path: &str) -> io::Result<FastUtxoSet> {
        Ok(FastUtxoSet {
            file: File::open(path)?,
        })
    }

    fn read_utxo(&mut self) -> io::Result<Utxo> {
        let mut tx_hash = [0; 32];
        self.file.read_exact(&mut tx_hash)?;
        let vout = self.file.read_u32::<LittleEndian>()?;
        let height_flagged = self.file.read_i32::<LittleEndian>()?;
        let flags = ((height_flagged & 0x0100_0000) >> 24) as u8;
        let block_height = height_flagged & 0x00ff_ffff;
        let amount = self.file.read_u64::<LittleEndian>()?;
        let script_len = self.file.read_u32::<LittleEndian>()?;
        let mut script = vec![0; script_len as usize];
        self.file.read_exact(&mut script)?;
        Ok(Utxo {
            outpoint: TxOutpoint {
                tx_hash, vout,
            },
            amount, script, block_height, flags,
        })
    }
}

impl Iterator for FastUtxoSet {
    type Item = io::Result<Utxo>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_utxo() {
            Ok(utxo) => Some(Ok(utxo)),
            Err(err) => {
                if err.kind() == io::ErrorKind::UnexpectedEof {
                    None
                } else {
                    Some(Err(err))
                }
            }
        }
    }
}

impl std::fmt::Display for Utxo {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> Result<(), std::fmt::Error> {
        writeln!(f, "Utxo: {}:{}", tx_hash_to_hex(&self.outpoint.tx_hash), self.outpoint.vout)?;
        writeln!(f, " amount:       {}", self.amount)?;
        writeln!(f, " script:       {}", hex::encode(&self.script))?;
        writeln!(f, " block_height: {}", self.block_height)?;
        writeln!(f, " flags:        {:x}", self.flags)?;
        Ok(())
    }
}
