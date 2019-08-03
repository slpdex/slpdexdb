use actix::prelude::*;
use slpdexdb_base::{BlockHeader, Error};
use futures::Future;
use std::convert::identity;

pub struct HeaderTip {
    pub header: BlockHeader,
    pub height: i32,
}

pub struct HeaderTipQuery;

impl Message for HeaderTipQuery {
    type Result = Result<HeaderTip, Error>;
}

pub struct AddHeaderQuery(BlockHeader);

impl Message for AddHeaderQuery {
    type Result = Result<(), Error>;
}

pub struct DbActor {
    header_tip_query: Recipient<HeaderTipQuery>,
    add_header_query: Recipient<AddHeaderQuery>,
}

impl Actor for DbActor {
    type Context = Context<DbActor>;
}

impl Handler<HeaderTipQuery> for DbActor {
    type Result = Response<HeaderTip, Error>;

    fn handle(&mut self, msg: HeaderTipQuery, _ctx: &mut Self::Context) -> Self::Result {
        Response::fut(self.header_tip_query.send(msg).from_err().and_then(identity))
    }
}

impl Handler<AddHeaderQuery> for DbActor {
    type Result = Response<(), Error>;

    fn handle(&mut self, msg: AddHeaderQuery, _ctx: &mut Self::Context) -> Self::Result {
        Response::fut(self.add_header_query.send(msg).from_err().and_then(identity))
    }
}
