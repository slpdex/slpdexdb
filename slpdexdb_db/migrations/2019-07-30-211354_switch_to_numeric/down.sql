ALTER TABLE token
    ALTER COLUMN "initial_supply" TYPE BIGINT,
    ALTER COLUMN "current_supply" TYPE BIGINT;

ALTER TABLE tx_output
    ALTER COLUMN "value_token_base" TYPE BIGINT;

ALTER TABLE trade_offer
    DROP COLUMN "price_per_token",
    DROP COLUMN "is_inverted",
    ADD COLUMN "approx_price_per_token" DOUBLE PRECISION,
    ADD COLUMN "price_per_token_numer" BIGINT,
    ADD COLUMN "price_per_token_denom" BIGINT;

UPDATE trade_offer SET "approx_price_per_token" = 0, "price_per_token_numer" = 0, "price_per_token_denom" = 0;

ALTER TABLE trade_offer
    ALTER COLUMN "approx_price_per_token" SET NOT NULL,
    ALTER COLUMN "price_per_token_numer" SET NOT NULL,
    ALTER COLUMN "price_per_token_denom" SET NOT NULL;
