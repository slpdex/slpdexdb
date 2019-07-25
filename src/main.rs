#[macro_use]
extern crate diesel;

mod block;
mod message_error;
mod message_header;
mod message;
mod version_message;
mod inv_message;
mod get_headers_message;
mod headers_message;
mod node;
mod db;
mod fast_utxo;
mod tx_source;
mod tx_history;
mod config;

mod models;
mod schema;

use hex_literal::hex;


fn main() -> Result<(), Box<std::error::Error>> {
/*    tx_source::TxSource::new().request_txs(&[
        //tx_source::TxFilter::TokenId(hex!("28022a6d389f3ecd5ae96fb3bc63083e95d2f2ebbffdb544fe186125640eb117")),
        tx_source::TxFilter::Address(cashcontracts::Address::from_cash_addr("bitcoincash:qq5lzj2p3kznpdsm06ms7la9g6d8hezkkg4mgq9rdh".to_string()).unwrap()),
    ]);
    return Ok(());
  */  //let fast = fast_utxo::FastUtxoSet::new("/Users/tobiasruck/workspace/bitcoin/slpdex-backend/data/QmXkBQJrMKkCKNbwv4m5xtnqwU9Sq7kucPigvZW8mWxcrv")?;
    //for utxo in fast.take(10) {
    //    let utxo = utxo?;
    //    println!("{}", utxo);
    //}
    use diesel::prelude::*;
    use diesel::pg::PgConnection;

    let connection_str = std::env::var("DATABASE_STR")?;

    let db_conn = PgConnection::establish(&connection_str)?;



    let mut node = node::Node::new(db::Db::new(db_conn));
    node.connect("100.1.209.114:8333")?;
    node.run_forever()?;
    Ok(())
}
