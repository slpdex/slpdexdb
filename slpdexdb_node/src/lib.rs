pub mod messages;
mod message_packet;
mod message_error;
mod message_header;
mod message;
mod codec;
pub mod actors;
mod db_query;
mod msg;

pub use message_packet::*;
pub use message_error::*;
pub use message_error::*;
pub use message::*;
pub use db_query::*;


use std::str::FromStr;
use actix::prelude::*;
use tokio_tcp::TcpStream;
use tokio_codec::FramedRead;
use std::net;

use crate::codec::MessageCodec;
use crate::actors::{NodeActor, VersionActor, InvActor};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    Ok(actix::System::run(|| {
        let addr = net::SocketAddr::from_str("100.1.209.114:8333").unwrap();
        Arbiter::spawn(
            TcpStream::connect(&addr)
                .and_then(|stream| {
                    println!("connected");
                    let addr = NodeActor::create_from_tcp(stream);
                    futures::future::ok(())
                })
                .map_err(|err| {
                    eprintln!("Connect failed: {}", err);
                })
        )
    })?)
}
