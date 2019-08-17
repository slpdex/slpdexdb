use crate::tx_source::{TxFilter, SortKey};
use crate::tx_history::TxHistory;
use crate::token::Token;
use cashcontracts::{Address, AddressType, tx_hash_to_hex};
use crate::data::tx_hash_from_slice;


#[derive(Copy, Clone, FromPrimitive, Debug)]
pub enum UpdateSubjectType {
    Token = 1,
    Exch = 2,
    AddressHistory = 3,
    AddressUTXOs = 4,
    TokenStats = 5,
}

#[derive(Clone, Debug)]
pub struct UpdateSubject {
    pub subject_type: UpdateSubjectType,
    pub hash: Option<Vec<u8>>,
    pub is_confirmed: bool,
}

#[derive(Clone, Debug)]
pub struct UpdateHistory {
    pub last_height:   i32,
    pub last_tx_hash:  Option<Vec<u8>>,
    pub subject:       UpdateSubject,
    pub completed:     bool,
}

impl UpdateHistory {
    pub fn next_filters(&self) -> Vec<TxFilter> {
        use self::UpdateSubjectType::*;
        let mut filters = vec![
            TxFilter::SortBy(SortKey::TxHash),
        ];
        match &self.last_tx_hash {
            Some(last_hash) if !self.completed => {
                let mut tx_hash = [0; 32];
                tx_hash.copy_from_slice(&last_hash);
                filters.append(&mut vec![
                    TxFilter::MinTxHash(tx_hash),
                ]);
            },
            _ => {
                filters.push(TxFilter::MinBlockHeight(self.last_height));
            },
        };
        match self.subject.subject_type {
            Token => {},
            Exch => {
                filters.push(TxFilter::Exch);
            },
            AddressHistory => {
                let mut address_hash = [0; 20];
                address_hash.copy_from_slice(
                    self.subject.hash.as_ref().expect("Subject hash must be present for AddressHistory")
                );
                filters.push(TxFilter::Address(Address::from_bytes(AddressType::P2PKH, address_hash)));
            },
            _ => unimplemented!(),
        };
        filters
    }

    pub fn initial(subject: UpdateSubject) -> Self {
        UpdateHistory {
            last_height: 0,
            last_tx_hash: None,
            subject,
            completed: true,
        }
    }

    pub fn from_tx_history(tx_history: &TxHistory,
                           subject: UpdateSubject,
                           current_height: i32) -> Self {
        UpdateHistory {
            last_height: tx_history.txs.iter()
                .filter_map(|tx| tx.height)
                .max()
                .unwrap_or(current_height),
            last_tx_hash: tx_history.txs.last().map(|tx| tx.hash.to_vec()),
            subject,
            completed: tx_history.txs.is_empty(),
        }
    }

    pub fn from_tokens(tokens: &[Token], current_height: i32) -> Self {
        UpdateHistory {
            last_height: tokens.iter().map(|token| token.block_created_height)
                .max().unwrap_or(current_height),
            last_tx_hash: tokens.last().map(|token| token.hash.to_vec()),
            subject: UpdateSubject {
                subject_type: UpdateSubjectType::Token,
                hash: None,
                is_confirmed: true,
            },
            completed: tokens.is_empty(),
        }
    }
}

impl std::fmt::Display for UpdateHistory {
    fn fmt<'a>(&self, f: &mut std::fmt::Formatter<'a>) -> Result<(), std::fmt::Error> {
        writeln!(f, "last_height: {}", self.last_height)?;
        writeln!(f, "last_tx_hash: {:?}", self.last_tx_hash.as_ref().map(|tx_hash| tx_hash_from_slice(tx_hash)).as_ref().map(tx_hash_to_hex))?;
        writeln!(f, "subject_type: {:?}", self.subject.subject_type)?;
        writeln!(f, "subject_hash: {:?}", self.subject.hash.as_ref().map(hex::encode))?;
        writeln!(f, "completed: {}", self.completed)?;
        writeln!(f, "is_confirmed: {}", self.subject.is_confirmed)?;
        Ok(())
    }
}
