-- Manual cash entries. bank_executed stays false; a status flag is not this ledger.
create table if not exists cash_entries (
  tenant_id uuid not null,
  entry_id uuid not null,
  direction text not null,
  partner_code text not null,
  currency text not null,
  account_code text not null,
  amount_cent bigint not null,
  bank_executed boolean not null default false,
  reverses_entry_id uuid,
  created_at timestamptz not null default now(),
  primary key (tenant_id, entry_id),
  constraint cash_entries_direction_check check (direction in ('RECEIPT', 'PAYMENT', 'REFUND', 'REVERSAL'))
);
