pub struct Endpoint {
    pub bitdb_endpoint_url: String,
    pub slpdb_endpoint_url: String,
}

impl Endpoint {
    pub fn new() -> Self {
        Endpoint {
            bitdb_endpoint_url: "https://bitdb.bch.sx/q/".to_string(),
            slpdb_endpoint_url: "https://slpdb.fountainhead.cash/q/".to_string(),
        }
    }
}
