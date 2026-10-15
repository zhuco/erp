-- sqlx created the original cost fingerprints in UTC. A session display timezone
-- must not invalidate immutable sources or require rewriting historical hashes.
alter function receipt_value_legacy_facts(uuid,uuid) set timezone to 'UTC';
