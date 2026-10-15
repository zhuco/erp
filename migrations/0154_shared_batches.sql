-- A batch is one enterprise, item, and batch code. Serials are whole pieces. Trace uses actual consumption.

create table if not exists shared_batches (
  tenant_id uuid not null,
  batch_id uuid not null,
  item_code text not null,
  batch_code text not null,
  produced_on text not null,
  expires_on text not null,
  frozen boolean not null default false,
  serialized boolean not null default false,
  created_at timestamptz not null default now(),
  primary key (tenant_id, batch_id),
  unique (tenant_id, item_code, batch_code)
);

create table if not exists shared_serials (
  tenant_id uuid not null,
  serial_id uuid not null,
  item_code text not null,
  batch_code text not null,
  serial_no text not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, serial_id),
  unique (tenant_id, item_code, serial_no)
);

create table if not exists shared_batch_uses (
  tenant_id uuid not null,
  use_id uuid not null,
  input_batch text not null,
  output_batch text not null,
  qty_micros bigint not null,
  work_order text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, use_id)
);
