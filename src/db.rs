use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::block::BlockHeader;

use crate::{models, schema::*};

pub struct Db {
    connection: PgConnection,
}

impl Db {
    pub fn new(connection: PgConnection) -> Self {
        Db { connection }
    }

    pub fn add_header(&self, header: &BlockHeader) -> QueryResult<()> {
        let height = if header.prev_block == [0; 32] {
            0
        } else {
            let blocks = blocks::table
                .filter(blocks::hash.eq(header.prev_block.as_ref()))
                .load::<models::Block>(&self.connection)?;
            blocks[0].height + 1
        };
        diesel::insert_into(blocks::table)
            .values(&models::Block::from_block_header(header, height))
            .execute(&self.connection)?;
        Ok(())
    }

    pub fn header_tips(&self, n_recent: i64) -> QueryResult<Vec<BlockHeader>> {
        Ok(blocks::table
            .order(blocks::height.desc())
            .limit(n_recent)
            .load::<models::Block>(&self.connection)?
            .into_iter()
            .map(|block: models::Block| block.to_block_header())
            .collect())
    }

    pub fn header_tip(&self) -> QueryResult<Option<BlockHeader>> {
        let mut tips = self.header_tips(1)?;
        if tips.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(tips.remove(0)))
        }
    }
}
