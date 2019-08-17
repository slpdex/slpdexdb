ALTER TABLE update_history
    ADD COLUMN "is_confirmed" BOOL;

UPDATE update_history SET "is_confirmed" = false;

ALTER TABLE update_history
    ALTER COLUMN "is_confirmed" SET NOT NULL;
