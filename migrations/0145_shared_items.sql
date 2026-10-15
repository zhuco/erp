-- One item may be bought, made, and sold. A referenced item cannot be deleted.

create table if not exists shared_items (
  tenant_id uuid not null,
  item_id uuid not null,
  item_code text not null,
  name text not null,
  spec text not null default '',
  category text not null default '',
  barcode text not null default '',
  buy boolean not null,
  make boolean not null,
  sell boolean not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, item_id),
  unique (tenant_id, item_code)
);

create table if not exists shared_item_refs (
  tenant_id uuid not null,
  item_id uuid not null,
  document_id text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, item_id, document_id)
);
