-- A purchase revision is a new snapshot. Receipt remainder is calculated from the source line.

create table if not exists shared_purchase_revisions (
  tenant_id uuid not null,
  revision_id uuid not null,
  po_id uuid not null,
  version_no integer not null,
  status text not null,
  supplier_code text not null,
  warehouse_code text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, revision_id),
  unique (tenant_id, po_id, version_no)
);

create table if not exists shared_purchase_lines (
  tenant_id uuid not null,
  revision_id uuid not null,
  line_no integer not null,
  item_code text not null,
  qty_micros bigint not null,
  price_cent bigint not null,
  tax_cent bigint not null,
  due_date text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, revision_id, line_no)
);

create table if not exists shared_purchase_receipts (
  tenant_id uuid not null,
  receipt_id uuid not null,
  po_id uuid not null,
  line_no integer not null,
  qty_micros bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, receipt_id)
);
