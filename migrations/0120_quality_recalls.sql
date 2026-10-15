-- Recall scope is frozen to a confirmed trace and one sampling-rule version.
create table if not exists mfg_sampling_decisions (
  tenant_id uuid not null,
  decision_id uuid not null,
  rule_code text not null,
  rule_version integer not null,
  period_code text not null,
  sample_size bigint not null,
  defects bigint not null,
  accept_number bigint not null,
  accepted boolean not null,
  defect_ppm bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, decision_id)
);

create table if not exists mfg_recalls (
  tenant_id uuid not null,
  recall_id uuid not null,
  input_batch text not null,
  rule_code text not null,
  rule_version integer not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, recall_id)
);

create table if not exists mfg_recall_items (
  tenant_id uuid not null,
  recall_id uuid not null,
  item_kind text not null,
  item_code text not null,
  qty_micros bigint not null,
  freeze_status text not null,
  notice_status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, recall_id, item_kind, item_code),
  constraint mfg_recall_items_kind_check check (item_kind in ('OUTPUT', 'CUSTOMER', 'STOCK'))
);
