-- PostgreSQL cache table for infracost pricing data
CREATE TABLE IF NOT EXISTS infracost_cache (
    key TEXT PRIMARY KEY NOT NULL,
    data TEXT NOT NULL,
    expires_at BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_infracost_cache_expires_at ON infracost_cache(expires_at);
