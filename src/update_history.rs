use crate::tx_source::{TxFilter, SortKey};
use crate::tx_history::TxHistory;

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
    pub last_hash:     Option<Vec<u8>>,
    pub subject_type:  UpdateSubjectType,
    pub completed:     bool,
}

impl UpdateHistory {
    pub fn next_filters(&self) -> Vec<TxFilter> {
        use self::UpdateSubjectType::*;
        match self.subject_type {
            Exch => {
                let mut filters = vec![
                    TxFilter::Exch,
                    TxFilter::MinBlockHeight(self.last_height),
                ];
                match &self.last_hash {
                    Some(last_hash) if !self.completed => {
                        let mut tx_hash = [0; 32];
                        tx_hash.copy_from_slice(&last_hash);
                        filters.append(&mut vec![
                            TxFilter::MinTxHash(tx_hash),
                            TxFilter::SortBy(SortKey::TxHash),
                        ]);
                    },
                    _ => {},
                }
                filters
            },
            _ => unimplemented!(),
        }
    }

    pub fn initial(subject_type: UpdateSubjectType) -> Self {
        UpdateHistory {
            last_height: 0,
            last_hash: None,
            subject_type,
            completed: true,
        }
    }

    pub fn from_tx_history(tx_history: &TxHistory,
                           subject_type: UpdateSubjectType,
                           current_height: i32) -> Self {
        UpdateHistory {
            last_height: tx_history.txs.iter()
                .filter_map(|tx| tx.height)
                .max()
                .unwrap_or(current_height),
            last_hash: tx_history.txs.last().map(|tx| tx.hash.to_vec()),
            subject_type,
            completed: tx_history.txs.is_empty(),
        }
    }
}
