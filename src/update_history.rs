use crate::tx_source::{TxFilter, SortKey};
use crate::tx_history::TxHistory;
use cashcontracts::{Address, AddressType};


#[derive(Copy, Clone, FromPrimitive)]
pub enum UpdateSubjectType {
    Token = 1,
    Exch = 2,
    AddressHistory = 3,
    AddressUTXOs = 4,
    TokenStats = 5,
}

pub struct UpdateHistory {
    pub last_height:   i32,
    pub last_tx_hash:  Option<Vec<u8>>,
    pub subject_type:  UpdateSubjectType,
    pub subject_hash:  Option<Vec<u8>>,
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
        match self.subject_type {
            Exch => {
                filters.push(TxFilter::Exch);
            },
            AddressHistory => {
                let mut address_hash = [0; 20];
                address_hash.copy_from_slice(
                    self.subject_hash.as_ref().expect("Subject hash must be present for AddressHistory")
                );
                filters.push(TxFilter::Address(Address::from_bytes(AddressType::P2PKH, address_hash)));
            },
            _ => unimplemented!(),
        };
        filters
    }

    pub fn initial(subject_type: UpdateSubjectType, subject_hash: Option<Vec<u8>>) -> Self {
        UpdateHistory {
            last_height: 0,
            last_tx_hash: None,
            subject_type,
            subject_hash,
            completed: true,
        }
    }

    pub fn from_tx_history(tx_history: &TxHistory,
                           subject_type: UpdateSubjectType,
                           subject_hash: Option<Vec<u8>>,
                           current_height: i32) -> Self {
        UpdateHistory {
            last_height: tx_history.txs.iter()
                .filter_map(|tx| tx.height)
                .max()
                .unwrap_or(current_height),
            last_tx_hash: tx_history.txs.last().map(|tx| tx.hash.to_vec()),
            subject_type,
            subject_hash,
            completed: tx_history.txs.is_empty(),
        }
    }
}