use actix::prelude::*;
use diesel::prelude::*;
use slpdexdb_base::{Error, GENESIS};
use slpdexdb_db::{Pool, Db};
use slpdexdb_node::{HeaderTipQuery, HeaderTip, AddHeadersQuery};


pub struct DbActor {
    db: Db,
    //pool: Pool,
}

impl DbActor {
    pub fn create() -> Result<Addr<Self>, Error> {
        let connection_str = std::env::var("DATABASE_URL").expect("DATABASE_URL");
        let connection = PgConnection::establish(&connection_str)?;
        let db = Db::new(connection);
        Ok(DbActor::start(DbActor { db }))
    }
}

impl Actor for DbActor {
    type Context = Context<Self>;
}

impl Handler<HeaderTipQuery> for DbActor {
    type Result = Result<HeaderTip, Error>;

    fn handle(&mut self, _msg: HeaderTipQuery, _ctx: &mut Self::Context) -> Self::Result {
        let result = self.db.header_tip()?;
        Ok(result
            .map(|(header, height)| {
                HeaderTip { header, height }
            })
            .unwrap_or_else(|| HeaderTip { header: GENESIS, height: 0 }))
    }
}

impl Handler<AddHeadersQuery> for DbActor {
    type Result = Result<(), Error>;

    fn handle(&mut self, msg: AddHeadersQuery, _ctx: &mut Self::Context) -> Self::Result {
        self.db.add_headers(&msg.0)?;
        Ok(())
    }
}
