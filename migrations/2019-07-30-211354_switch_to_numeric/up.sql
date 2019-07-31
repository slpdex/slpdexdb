ALTER TABLE token
    ALTER COLUMN "initial_supply" TYPE NUMERIC(26),
    ALTER COLUMN "current_supply" TYPE NUMERIC(26);

ALTER TABLE tx_output
    ALTER COLUMN "value_token_base" TYPE NUMERIC(26);

ALTER TABLE trade_offer
    ALTER COLUMN "sell_amount_token_base" TYPE NUMERIC(26),
    DROP COLUMN "approx_price_per_token",
    DROP COLUMN "price_per_token_numer",
    DROP COLUMN "price_per_token_denom",
    ADD COLUMN "price_per_token" NUMERIC(52, 26),
    ADD COLUMN "is_inverted" BOOL;

UPDATE trade_offer SET "price_per_token" = 0, "is_inverted" = false;

ALTER TABLE trade_offer
    ALTER COLUMN "price_per_token" SET NOT NULL,
    ALTER COLUMN "is_inverted" SET NOT NULL;
