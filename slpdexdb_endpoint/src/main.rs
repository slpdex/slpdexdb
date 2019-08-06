mod actors;
mod query;
mod msg;

use std::str::FromStr;
use std::net;
use tokio_tcp::TcpStream;
use actix::prelude::*;
use diesel::prelude::*;

use actix_web::{middleware, web, App, HttpResponse, HttpRequest, HttpServer, Responder};
use actix_web_actors::ws;

use cashcontracts::{Address, tx_hash_to_hex};
use slpdexdb_node::actors as node_actors;
use slpdexdb_base::SLPDEXConfig;
use slpdexdb_db::Db;
use crate::actors::{TxActor, ResyncActor, PeersActor, WsActor};
use crate::msg::ConnectToPeer;

pub fn connect_db() -> Db {
    let connection_str = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let connection = PgConnection::establish(&connection_str).unwrap();
    Db::new(connection)
}

fn index(r: HttpRequest,
         stream: web::Payload,
         path: web::Path<(String,)>,
         tx: web::Data<Addr<TxActor>>) -> Result<HttpResponse, actix_web::Error> {
    let address_str = &path.0;
    eprintln!("connect to address {}", address_str);
    let address = Address::from_cash_addr(address_str.clone()).unwrap();  // TODO: handle error
    //Ok(HttpResponse::Ok().body("Hello"))
    ws::start(WsActor::new(address, tx.get_ref().clone()), &r, stream)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();
    let port = std::env::var("PORT").unwrap_or("7501".to_string());
    actix::System::run(move || {
        let resync_addr = SyncArbiter::start(2, || {
            ResyncActor::new(connect_db(), SLPDEXConfig::default())
        });
        let db_addr = actors::DbActor::create().unwrap();
        let db_addr = slpdexdb_node::DbActor::start(slpdexdb_node::DbActor {
            add_header_query: db_addr.clone().recipient(),
            header_tip_query: db_addr.recipient(),
        });
        let tx_addr = TxActor::start(TxActor::new(connect_db(), SLPDEXConfig::default(), resync_addr));
        let peers_addr = PeersActor::start(PeersActor::new(tx_addr.clone(), db_addr));
        let socket_addr = net::SocketAddr::from_str("100.1.209.114:8333").unwrap();
        peers_addr.do_send(ConnectToPeer { socket_addr });

        HttpServer::new(move || {
            App::new()
                .wrap(middleware::Logger::default())
                .data(tx_addr.clone())
                .service(
                    web::resource("/address/{address}").route(web::get().to(index))
                )
        })
            .bind(format!("127.0.0.1:{}", port)).unwrap()
            .start();
    })?;
    Ok(())
}
