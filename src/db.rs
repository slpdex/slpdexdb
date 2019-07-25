use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::block::BlockHeader;
use crate::{models, schema::*};

use std::collections::{HashMap, BTreeSet};

pub struct Db {
    connection: PgConnection,
}

impl Db {
    pub fn new(connection: PgConnection) -> Self {
        Db { connection }
    }

    pub fn add_headers(&self, headers: &[BlockHeader]) -> QueryResult<()> {
        let mut remaining_visit = (0..headers.len()).into_iter().collect::<BTreeSet<_>>();
        let mut heights = self.header_tips(10)?
            .iter()
            .map(|(header, height)| (header.hash(), *height))
            .collect::<HashMap<_, _>>();
        loop {
            let tuple = remaining_visit
                .iter().cloned()
                .find_map(|i|
                    if headers[i].prev_block != [0; 32] {
                        heights.get(&headers[i].prev_block).map(|i| *i + 1)
                    } else {
                        Some(0)
                    }.map(|height| (i, &headers[i], height))
                );
            match tuple {
                Some((i, header, height)) => {
                    heights.insert(header.hash(), height);
                    remaining_visit.remove(&i);
                },
                None => break,
            };
        }
        let db_blocks = headers.iter().map(|header| {
            models::Block::from_block_header(header, heights[&header.hash()])
        }).collect::<Vec<_>>();
        diesel::insert_into(blocks::table)
            .values(&db_blocks)
            .execute(&self.connection)?;
        Ok(())
    }

    pub fn header_tips(&self, n_recent: i64) -> QueryResult<Vec<(BlockHeader, i32)>> {
        Ok(blocks::table
            .order(blocks::height.desc())
            .limit(n_recent)
            .load::<models::Block>(&self.connection)?
            .into_iter()
            .map(|block: models::Block| (block.to_block_header(), block.height))
            .collect())
    }

    pub fn header_tip(&self) -> QueryResult<Option<(BlockHeader, i32)>> {
        let mut tips = self.header_tips(1)?;
        if tips.len() == 0 {
            Ok(None)
        } else {
            Ok(Some(tips.remove(0)))
        }
    }
}
