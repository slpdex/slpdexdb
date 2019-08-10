use std::time::{SystemTime, UNIX_EPOCH};
use actix::prelude::*;
use cashcontracts::Address;
use slpdexdb_base::{Error, SLPDEXConfig};
use slpdexdb_db::{Db, TxSource, TokenSource, UpdateSubjectType, UpdateHistory, TxHistory, Token, OutputType};
use crate::msg::{ResyncAddress, ProcessTransactions, NewTransactions};


fn _resync(db: &Db, config: &SLPDEXConfig) -> Result<(), Error> {
    _resync_tokens(db)?;
    _resync_trade_offers(db, config)?;
    Ok(())
}

fn _resync_tokens(db: &Db) -> Result<(), Error> {
    let token_source = TokenSource::new();
    loop {
        let current_height = db.header_tip()?.map(|(_, height)| height).unwrap_or(0);
        let last_update =
            db.last_update(UpdateSubjectType::Token, None)?
                .unwrap_or(UpdateHistory::initial(UpdateSubjectType::Token, None));
        let token_entries = token_source.request_tokens(&last_update.next_filters())?;
        let tokens = token_entries.into_iter()
            .filter_map(|token_entry| {
                Token::from_entry(token_entry).map_err(|err| eprintln!("token error: {}", err)).ok()
            })
            .collect::<Vec<_>>();
        if tokens.len() == 0 {
            break
        }
        for token in tokens.iter() {
            println!("try adding token {:?}", token);
            println!("document_uri: {:?}", token.document_uri.as_ref().map(|x| hex::encode(x.as_bytes())));
            db.add_tokens(&[token.clone()])?;
        }
        db.add_update_history(&UpdateHistory::from_tokens(&tokens, current_height))?;
    }
    Ok(())
}

fn _resync_trade_offers(db: &Db, config: &SLPDEXConfig) -> Result<(), Error> {
    let tx_source = TxSource::new();
    loop {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let current_height = db.header_tip()?.map(|(_, height)| height).unwrap_or(0);
        let last_update =
            db.last_update(UpdateSubjectType::Exch, None)?
                .unwrap_or(UpdateHistory::initial(UpdateSubjectType::Exch, None));
        let tx_entries = tx_source.request_txs(&last_update.next_filters(), config)?;
        let history = TxHistory::from_entries(&tx_entries, timestamp as i64, config);
        if history.txs.len() == 0 {
            break
        }
        db.add_tx_history(&history)?;
        db.add_update_history(
            &UpdateHistory::from_tx_history(&history, UpdateSubjectType::Exch, None, current_height)
        )?;
    }
    db.update_utxo_set_exch()?;
    Ok(())
}

fn _resync_address(db: &Db, config: &SLPDEXConfig, address: &Address) -> Result<(), Error> {
    loop {
        let tx_source = TxSource::new();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let current_height = db.header_tip()?.map(|(_, height)| height).unwrap_or(0);
        let last_update =
            db.last_update(UpdateSubjectType::AddressHistory, Some(address.bytes().to_vec()))?
                .unwrap_or(UpdateHistory::initial(
                    UpdateSubjectType::AddressHistory,
                    Some(address.bytes().to_vec()),
                ));
        println!("last update: {}", last_update);
        let tx_entries = tx_source.request_txs(&last_update.next_filters(), config)?;
        let history = TxHistory::from_entries(&tx_entries, timestamp as i64, config);
        if history.txs.len() > 0 {
            db.add_tx_history(&history)?;
        }
        db.add_update_history(
            &UpdateHistory::from_tx_history(
                &history,
                UpdateSubjectType::AddressHistory,
                Some(address.bytes().to_vec()),
                current_height,
            )
        )?;
        if history.txs.len() == 0 {
            break
        }
    }
    db.update_utxo_set(&address)?;
    Ok(())
}

pub struct ResyncActor {
    db: Db,
    config: SLPDEXConfig,
}

impl ResyncActor {
    pub fn new(db: Db, config: SLPDEXConfig) -> Self {
        ResyncActor { db, config }
    }
}

impl Actor for ResyncActor {
    type Context = SyncContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        _resync(&self.db, &self.config)
            .map_err(|err| eprintln!("resync failed: {}", err))
            .unwrap_or(());
    }
}

impl Handler<ResyncAddress> for ResyncActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: ResyncAddress, _ctx: &mut Self::Context) -> Self::Result {
        let address = msg.0;
        _resync_address(&self.db, &self.config, &address)
    }
}
