-- 0034: Event reliability hardening (outbox publish + worker idempotency)

alter table if exists outbox_events
  add column if not exists dedup_key text,
  add column if not exists last_attempt_at timestamptz not null default now(),
  add column if not exists published_at timestamptz;

create index if not exists idx_outbox_dispatching_stale
  on outbox_events(status, last_attempt_at);

create unique index if not exists idx_outbox_dedup_key
  on outbox_events(tenant_id, event_month, dedup_key)
  where dedup_key is not null;

create table if not exists worker_effects (
  tenant_id uuid not null references tenants(tenant_id),
  outbox_id uuid not null,
  event_type text not null,
  status text not null default 'PROCESSING', -- PROCESSING/SUCCEEDED/FAILED
  last_error text not null default '',
  processed_at timestamptz not null default now(),
  primary key (tenant_id, outbox_id)
);

create index if not exists idx_worker_effects_tenant_event
  on worker_effects(tenant_id, event_type, processed_at desc);
