
CREATE TABLE token (
    "id"                   SERIAL PRIMARY KEY,
    "hash"                 BYTEA    NOT NULL,
    "decimals"             INT      NOT NULL,
    "timestamp"            BIGINT   NOT NULL,
    "version_type"         SMALLINT NOT NULL,
    "document_uri"         VARCHAR(200),
    "symbol"               VARCHAR(200),
    "name"                 VARCHAR(200),
    "document_hash"        VARCHAR(200),
    "initial_supply"       BIGINT NOT NULL,
    "current_supply"       BIGINT NOT NULL,
    "block_created_height" INT NOT NULL
);

CREATE TABLE tx (
    "id"        BIGSERIAL PRIMARY KEY,
    "hash"      BYTEA NOT NULL,
    "height"    INT NOT NULL,
    "timestamp" BIGINT NOT NULL,
    "tx_type"   INT NOT NULL
);

CREATE TABLE slp_tx (
    "tx"      BIGINT PRIMARY KEY REFERENCES tx ("id") ON DELETE CASCADE,
    "token"   INT REFERENCES token ("id") ON DELETE RESTRICT,
    "version" INT NOT NULL,
    "slp_type" VARCHAR(14) NOT NULL
);

CREATE TABLE tx_output (
    "tx"               BIGINT REFERENCES tx (id) ON DELETE CASCADE,
    "idx"              INT NOT NULL,
    "value_satoshis"   BIGINT NOT NULL,
    "value_token_base" BIGINT NOT NULL,
    "address"          BYTEA,
    "output_type"      INT NOT NULL,
    PRIMARY KEY ("tx", "idx")
);

CREATE TABLE tx_input (
    "tx"         BIGINT REFERENCES tx (id) ON DELETE CASCADE,
    "idx"        INT NOT NULL,
    "output_tx"  BYTEA NOT NULL,
    "output_idx" INT NOT NULL,
    "address"    BYTEA,
    PRIMARY KEY ("tx", "idx")
);

CREATE TABLE trade_offer (
    "id"                     SERIAL PRIMARY KEY,
    "tx"                     BIGINT REFERENCES tx (id) ON DELETE CASCADE,
    "output_idx"             INT,
    "input_tx"               BYTEA NOT NULL,
    "input_idx"              INT NOT NULL,
    "approx_price_per_token" DOUBLE PRECISION NOT NULL,
    "price_per_token_numer"  BIGINT NOT NULL,
    "price_per_token_denom"  BIGINT NOT NULL,
    "script_price"           BIGINT NOT NULL,
    "sell_amount_token_base" BIGINT NOT NULL,
    "receiving_address"      BYTEA NOT NULL
);
