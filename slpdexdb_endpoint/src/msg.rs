use actix::prelude::*;
use cashcontracts::Address;
use slpdexdb_base::Error;

pub struct SetAddressActive(pub Address, pub bool);

impl Message for SetAddressActive {
    type Result = Result<(), Error>;
}

pub struct ResyncAddress(pub Address);

impl Message for ResyncAddress {
    type Result = Result<(), Error>;
}

