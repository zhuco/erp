-- 0011_dead_letters.sql
create table if not exists dead_letters (
  dead_letter_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  outbox_id uuid not null,
  event_type text not null,
  payload jsonb not null,
  error text not null,
  attempts int not null,
  classification text not null,
  next_action text not null,
  status text not null default 'PENDING', -- PENDING/REPLAYED
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  replayed_at timestamptz
);

create unique index if not exists idx_dead_letters_outbox
  on dead_letters(tenant_id, outbox_id);

create index if not exists idx_dead_letters_status
  on dead_letters(status, created_at desc);
