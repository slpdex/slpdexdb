CREATE TABLE utxo_address (
    "tx"          BIGINT NOT NULL,
    "idx"         INT NOT NULL,
    "address"     BYTEA,
    PRIMARY KEY ("tx", "idx"),
    FOREIGN KEY ("tx", "idx") REFERENCES tx_output ("tx", "idx") ON DELETE CASCADE
);

CREATE TABLE utxo_trade_offer (
    "tx"          BIGINT NOT NULL,
    "idx"         INT NOT NULL,
    PRIMARY KEY ("tx", "idx"),
    FOREIGN KEY ("tx", "idx") REFERENCES tx_output ("tx", "idx") ON DELETE CASCADE
);
