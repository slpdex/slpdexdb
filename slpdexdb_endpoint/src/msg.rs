use actix::prelude::*;
use cashcontracts::Address;
use slpdexdb_base::Error;
use std::net;
use slpdexdb_db::{Utxo, TxDelta};

pub struct ConnectToPeer {
    pub socket_addr: net::SocketAddr,
}

impl Message for ConnectToPeer {
    type Result = Result<(), Error>;
}

pub struct ActivateAddress(pub Address);

impl Message for ActivateAddress {
    type Result = Result<(), Error>;
}

pub struct DeactivateAddress(pub Address);

impl Message for DeactivateAddress {
    type Result = Result<(), Error>;
}

pub struct ResyncAddress(pub Address);

impl Message for ResyncAddress {
    type Result = Result<(), Error>;
}

pub struct FetchAddressUtxos(pub Address);

impl Message for FetchAddressUtxos {
    type Result = Result<Vec<Utxo>, Error>;
}

pub struct FetchAddressTxDeltas(pub Address);

impl Message for FetchAddressTxDeltas {
    type Result = Result<Vec<TxDelta>, Error>;
}
