CREATE TABLE update_history (
    "id"              BIGSERIAL PRIMARY KEY,
    "last_height"     INT NOT NULL,
    "last_tx_hash"    BYTEA,
    "last_tx_hash_be" BYTEA,
    "subject_type"    INT NOT NULL,
    "subject_hash"    BYTEA,
    "timestamp"       TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "completed"       BOOL NOT NULL
);
