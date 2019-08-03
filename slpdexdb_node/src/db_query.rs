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

pub struct AddHeadersQuery(pub Vec<BlockHeader>);

impl Message for AddHeadersQuery {
    type Result = Result<(), Error>;
}

pub struct DbActor {
    pub header_tip_query: Recipient<HeaderTipQuery>,
    pub add_header_query: Recipient<AddHeadersQuery>,
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

impl Handler<AddHeadersQuery> for DbActor {
    type Result = Response<(), Error>;

    fn handle(&mut self, msg: AddHeadersQuery, _ctx: &mut Self::Context) -> Self::Result {
        Response::fut(self.add_header_query.send(msg).from_err().and_then(identity))
    }
}
