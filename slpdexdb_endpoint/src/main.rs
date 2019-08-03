mod actors;

use std::str::FromStr;
use std::net;
use tokio_tcp::TcpStream;
use actix::prelude::*;

use slpdexdb_node::actors as node_actors;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(actix::System::run(|| {
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
                    let addr = node_actors::NodeActor::create_from_stream_db(stream, db_addr);
                    futures::future::ok(())
                })
                .map_err(|err| {
                    eprintln!("Connect failed: {}", err);
                })
        )
    })?)
}
