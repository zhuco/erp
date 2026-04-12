-- 0080: AI-4 after-sales + procurement + finance + observability updates

-- After-sales status backfill/defaults
alter table after_sales
  alter column after_sale_status set default 'SUBMITTED';
update after_sales
set after_sale_status = case status
  when 'OPEN' then 'SUBMITTED'
  when 'APPROVED' then 'ACCEPTED'
  when 'RECEIVED' then 'ACCEPTED'
  when 'REFUNDED' then 'CLOSED'
  when 'CLOSED' then 'CLOSED'
  when 'REJECTED' then 'REJECTED'
  else 'SUBMITTED'
end
where after_sale_status is null or after_sale_status = '';

alter table return_receipts
  alter column return_status set default 'WAITING_RETURN';
update return_receipts
set return_status = case status
  when 'RECEIVING' then 'QC_CHECKING'
  when 'PUTAWAY_DONE' then 'QC_PASSED'
  when 'CANCELLED' then 'QC_FAILED'
  else 'WAITING_RETURN'
end
where return_status is null or return_status = '';

alter table refund_docs
  alter column refund_status set default 'PENDING_APPROVAL';
update refund_docs
set refund_status = case status
  when 'PENDING' then 'PENDING_APPROVAL'
  when 'SENT' then 'PENDING_REFUND'
  when 'DONE' then 'REFUNDED'
  when 'FAILED' then 'REFUND_FAILED'
  else 'PENDING_APPROVAL'
end
where refund_status is null or refund_status = '';

create index if not exists idx_after_sales_tenant_after_sale_status
  on after_sales(tenant_id, after_sale_status, created_at desc);
create index if not exists idx_return_receipts_tenant_return_status
  on return_receipts(tenant_id, return_status, created_at desc);
create index if not exists idx_refund_docs_tenant_refund_status
  on refund_docs(tenant_id, refund_status, created_at desc);

-- Exchange docs
create table if not exists exchange_docs (
  exchange_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  after_sales_id uuid not null references after_sales(id) on delete cascade,
  exchange_status text not null default 'CREATED',
  outbound_doc_id uuid references outbound_docs(outbound_doc_id),
  shipment_id uuid references shipments(shipment_id),
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_exchange_docs_tenant_status
  on exchange_docs(tenant_id, exchange_status, created_at desc);
create index if not exists idx_exchange_docs_tenant_shipment
  on exchange_docs(tenant_id, shipment_id);

-- Procurement status backfill/defaults
alter table purchase_orders
  alter column purchase_order_status set default 'DRAFT';
update purchase_orders
set purchase_order_status = case status
  when 'DRAFT' then 'DRAFT'
  when 'SENT' then 'PENDING_APPROVAL'
  when 'CONFIRMED' then 'PLACED'
  when 'RECEIVING' then 'PARTIALLY_RECEIVED'
  when 'DONE' then 'RECEIVED'
  when 'CANCELLED' then 'CLOSED'
  else 'DRAFT'
end
where purchase_order_status is null or purchase_order_status = '';

alter table purchase_receipts
  alter column receiving_status set default 'RECEIVED';
update purchase_receipts pr
set receiving_status = case po.status
  when 'RECEIVING' then 'PART_RECEIVED'
  when 'DONE' then 'RECEIVED'
  else 'RECEIVED'
end
from purchase_orders po
where po.tenant_id = pr.tenant_id
  and po.po_id = pr.po_id
  and (pr.receiving_status is null or pr.receiving_status = '');

alter table ap_docs
  alter column settlement_status set default 'UNSETTLED';
update ap_docs
set settlement_status = case status
  when 'PAID' then 'SETTLED'
  else 'UNSETTLED'
end
where settlement_status is null or settlement_status = '';

create index if not exists idx_purchase_orders_tenant_purchase_status
  on purchase_orders(tenant_id, purchase_order_status, created_at desc);
create index if not exists idx_purchase_receipts_tenant_receiving_status
  on purchase_receipts(tenant_id, receiving_status, created_at desc);
create index if not exists idx_ap_docs_tenant_settlement_status
  on ap_docs(tenant_id, settlement_status, created_at desc);

-- Platform statements status rename
update platform_statements
set status = 'RECONCILED'
where status = 'DONE';

-- Reconcile diffs
create table if not exists reconcile_diffs (
  diff_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  job_id uuid not null references reconcile_jobs(job_id) on delete cascade,
  ref_type text not null,
  ref_id text not null,
  expected_cent bigint not null default 0,
  actual_cent bigint not null default 0,
  diff_cent bigint not null default 0,
  diff_reason text,
  status text not null default 'PENDING',
  raw_json jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, job_id, ref_type, ref_id)
);

create index if not exists idx_reconcile_diffs_tenant_job
  on reconcile_diffs(tenant_id, job_id);
create index if not exists idx_reconcile_diffs_tenant_status
  on reconcile_diffs(tenant_id, status, created_at desc);

-- Payment reconcile tables
create table if not exists payment_statements (
  statement_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  channel text not null,
  period_start date,
  period_end date,
  status text not null default 'IMPORTED',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, channel, period_start, period_end)
);

create index if not exists idx_payment_statements_tenant_status
  on payment_statements(tenant_id, status, created_at desc);

create table if not exists payment_statement_lines (
  tenant_id uuid not null references tenants(tenant_id),
  statement_id uuid not null references payment_statements(statement_id) on delete cascade,
  platform_order_id text not null,
  paid_cent bigint not null default 0,
  fee_cent bigint not null default 0,
  net_cent bigint not null default 0,
  occurred_at timestamptz,
  raw_json jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  primary key (tenant_id, statement_id, platform_order_id)
);

create index if not exists idx_payment_statement_lines_order
  on payment_statement_lines(tenant_id, platform_order_id);
create index if not exists idx_payment_statement_lines_statement
  on payment_statement_lines(tenant_id, statement_id);

create table if not exists payment_reconcile_jobs (
  job_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  statement_id uuid references payment_statements(statement_id) on delete set null,
  status text not null,
  diff_count bigint not null default 0,
  started_at timestamptz,
  finished_at timestamptz,
  last_error text,
  created_at timestamptz not null default now()
);

create index if not exists idx_payment_reconcile_jobs_tenant_status
  on payment_reconcile_jobs(tenant_id, status, created_at desc);

create table if not exists payment_reconcile_items (
  tenant_id uuid not null references tenants(tenant_id),
  job_id uuid not null references payment_reconcile_jobs(job_id) on delete cascade,
  platform_order_id text not null,
  order_id uuid,
  expected_cent bigint not null default 0,
  actual_cent bigint not null default 0,
  diff_cent bigint not null default 0,
  diff_reason text,
  raw_json jsonb not null default '{}'::jsonb,
  created_at timestamptz not null default now(),
  primary key (tenant_id, job_id, platform_order_id)
);

create index if not exists idx_payment_reconcile_items_tenant_job
  on payment_reconcile_items(tenant_id, job_id);
create index if not exists idx_payment_reconcile_items_diff
  on payment_reconcile_items(tenant_id, diff_cent);

create table if not exists writeoff_records (
  writeoff_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  ref_type text not null,
  ref_id text not null,
  amount_cent bigint not null default 0,
  reason text,
  created_by text,
  created_at timestamptz not null default now()
);

create index if not exists idx_writeoff_records_tenant_ref
  on writeoff_records(tenant_id, ref_type, ref_id);
