mod actors;
mod query;
mod msg;

use std::str::FromStr;
use std::net;
use tokio_tcp::TcpStream;
use actix::prelude::*;
use diesel::prelude::*;

use slpdexdb_node::actors as node_actors;
use slpdexdb_base::SLPDEXConfig;
use slpdexdb_db::Db;
use crate::actors::{TxActor, ResyncActor};


fn connect_db() -> Db {
    let connection_str = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let connection = PgConnection::establish(&connection_str).unwrap();
    Db::new(connection)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(actix::System::run(|| {
        let resync_addr = SyncArbiter::start(2, || {
            ResyncActor::new(connect_db(), SLPDEXConfig::default())
        });
        let db_addr = actors::DbActor::create().unwrap();
        let db_addr = slpdexdb_node::DbActor::start(slpdexdb_node::DbActor {
            add_header_query: db_addr.clone().recipient(),
            header_tip_query: db_addr.recipient(),
        });
        let addr = net::SocketAddr::from_str("100.1.209.114:8333").unwrap();
        Arbiter::spawn(
            TcpStream::connect(&addr)
                .and_then(move |stream| {
                    println!("connected");
                    let node_addr = node_actors::NodeActor::create_from_stream_db(stream, db_addr);
                    let tx_addr = TxActor::start(TxActor::new(
                        connect_db(),
                        SLPDEXConfig::default(),
                        node_addr,
                        resync_addr,
                    ));
                    futures::future::ok(())
                })
                .map_err(|err| {
                    eprintln!("Connect failed: {}", err);
                })
        )
    })?)
}
