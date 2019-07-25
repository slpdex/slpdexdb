pub struct SLPDEXConfig {
    pub fee_address: cashcontracts::Address,
    pub fee_divisor: u64,
    pub dust_limit: u64,
}

impl Default for SLPDEXConfig {
    fn default() -> Self {
        SLPDEXConfig {
            fee_address: cashcontracts::Address::from_cash_addr(
                "bitcoincash:qp5x5tmxluwm62ny66zy9u4zuqvkmcv8sq2ceuxmwd".to_string()
            ).unwrap(),
            fee_divisor: 500,
            dust_limit: 0x222,
        }
    }
}
