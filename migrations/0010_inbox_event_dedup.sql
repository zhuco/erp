-- 0010: Inbox event dedup uniqueness

create unique index if not exists idx_inbox_events_dedup
  on inbox_events(tenant_id, event_month, dedup_key);
