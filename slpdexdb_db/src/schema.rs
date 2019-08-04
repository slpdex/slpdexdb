table! {
    active_address (address) {
        address -> Bytea,
    }
}

table! {
    blocks (hash) {
        hash -> Bytea,
        height -> Int4,
        version -> Int4,
        prev_block -> Bytea,
        merkle_root -> Bytea,
        timestamp -> Int8,
        bits -> Int8,
        nonce -> Int8,
    }
}

table! {
    estimate (id) {
        id -> Int4,
        title -> Nullable<Text>,
        priority -> Nullable<Int4>,
        estimate_hours -> Nullable<Int4>,
        actual_hours -> Nullable<Int4>,
        deviation_reasons -> Nullable<Text>,
    }
}

table! {
    slp_tx (tx) {
        tx -> Int8,
        token -> Int4,
        version -> Int4,
        slp_type -> Varchar,
    }
}

table! {
    token (id) {
        id -> Int4,
        hash -> Bytea,
        decimals -> Int4,
        timestamp -> Int8,
        version_type -> Int2,
        document_uri -> Nullable<Varchar>,
        symbol -> Nullable<Varchar>,
        name -> Nullable<Varchar>,
        document_hash -> Nullable<Varchar>,
        initial_supply -> Numeric,
        current_supply -> Numeric,
        block_created_height -> Int4,
    }
}

table! {
    trade_offer (id) {
        id -> Int4,
        tx -> Int8,
        output_idx -> Nullable<Int4>,
        input_tx -> Bytea,
        input_idx -> Int4,
        script_price -> Int8,
        sell_amount_token_base -> Numeric,
        receiving_address -> Bytea,
        price_per_token -> Numeric,
        is_inverted -> Bool,
    }
}

table! {
    tx (id) {
        id -> Int8,
        hash -> Bytea,
        height -> Nullable<Int4>,
        timestamp -> Int8,
        tx_type -> Int4,
    }
}

table! {
    tx_input (tx, idx) {
        tx -> Int8,
        idx -> Int4,
        output_tx -> Bytea,
        output_idx -> Int4,
        address -> Nullable<Bytea>,
    }
}

table! {
    tx_output (tx, idx) {
        tx -> Int8,
        idx -> Int4,
        value_satoshis -> Int8,
        value_token_base -> Numeric,
        address -> Nullable<Bytea>,
        output_type -> Int4,
    }
}

table! {
    update_history (id) {
        id -> Int8,
        last_height -> Int4,
        last_tx_hash -> Nullable<Bytea>,
        last_tx_hash_be -> Nullable<Bytea>,
        subject_type -> Int4,
        subject_hash -> Nullable<Bytea>,
        timestamp -> Timestamptz,
        completed -> Bool,
    }
}

table! {
    utxo_address (tx, idx) {
        tx -> Int8,
        idx -> Int4,
        address -> Nullable<Bytea>,
    }
}

table! {
    utxo_trade_offer (tx, idx) {
        tx -> Int8,
        idx -> Int4,
    }
}

joinable!(slp_tx -> token (token));
joinable!(slp_tx -> tx (tx));
joinable!(trade_offer -> tx (tx));
joinable!(tx_input -> tx (tx));
joinable!(tx_output -> tx (tx));

allow_tables_to_appear_in_same_query!(
    active_address,
    blocks,
    estimate,
    slp_tx,
    token,
    trade_offer,
    tx,
    tx_input,
    tx_output,
    update_history,
    utxo_address,
    utxo_trade_offer,
);
