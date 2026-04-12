-- 0089: OMS recheck batch persistence + WMS outbound verify session scans

create table if not exists oms_recheck_batch_jobs (
  job_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  action text not null,
  reason text,
  status text not null default 'RUNNING',
  total_count bigint not null default 0,
  accepted_count bigint not null default 0,
  rejected_count bigint not null default 0,
  requested_by uuid,
  requested_at timestamptz not null default now(),
  completed_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists idx_oms_recheck_batch_jobs_tenant_created
  on oms_recheck_batch_jobs(tenant_id, created_at desc, job_id desc);
create index if not exists idx_oms_recheck_batch_jobs_tenant_status
  on oms_recheck_batch_jobs(tenant_id, status, created_at desc, job_id desc);

create table if not exists oms_recheck_batch_items (
  item_id uuid primary key default gen_random_uuid(),
  job_id uuid not null references oms_recheck_batch_jobs(job_id) on delete cascade,
  tenant_id uuid not null references tenants(tenant_id),
  order_id uuid,
  raw_order_id text not null,
  result text not null,
  error_code text,
  error_message text,
  before_json jsonb,
  after_json jsonb,
  processed_at timestamptz not null default now()
);

create index if not exists idx_oms_recheck_batch_items_job
  on oms_recheck_batch_items(job_id, processed_at asc, item_id asc);
create index if not exists idx_oms_recheck_batch_items_tenant_order
  on oms_recheck_batch_items(tenant_id, order_id, processed_at desc, item_id desc);

create table if not exists wms_outbound_verify_sessions (
  session_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
  shipment_no text not null,
  tracking_no text,
  package_no text not null,
  status text not null default 'VERIFYING',
  started_by uuid,
  started_at timestamptz not null default now(),
  finished_by uuid,
  finished_at timestamptz,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create unique index if not exists idx_wms_verify_sessions_active
  on wms_outbound_verify_sessions(tenant_id, outbound_doc_id, package_no)
  where status = 'VERIFYING';
create index if not exists idx_wms_verify_sessions_tenant_doc
  on wms_outbound_verify_sessions(tenant_id, outbound_doc_id, created_at desc, session_id desc);

create table if not exists wms_outbound_verify_scan_lines (
  line_id uuid primary key default gen_random_uuid(),
  session_id uuid not null references wms_outbound_verify_sessions(session_id) on delete cascade,
  tenant_id uuid not null references tenants(tenant_id),
  outbound_doc_id uuid not null references outbound_docs(outbound_doc_id) on delete cascade,
  package_no text not null,
  barcode text not null,
  sku_id uuid,
  qty bigint not null check (qty > 0),
  scanned_by uuid,
  scanned_at timestamptz not null default now(),
  extra_json jsonb not null default '{}'::jsonb
);

create index if not exists idx_wms_verify_scan_lines_session
  on wms_outbound_verify_scan_lines(session_id, scanned_at asc, line_id asc);
create index if not exists idx_wms_verify_scan_lines_tenant_doc
  on wms_outbound_verify_scan_lines(tenant_id, outbound_doc_id, scanned_at desc, line_id desc);
create index if not exists idx_wms_verify_scan_lines_tenant_sku
  on wms_outbound_verify_scan_lines(tenant_id, sku_id, scanned_at desc, line_id desc);
