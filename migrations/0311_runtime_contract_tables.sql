-- Install runtime dependencies previously created only by acceptance fixtures.

-- Existing compatibility rows are retained; tenant FKs constrain new writes.

create table if not exists attachment_download_links (
  tenant_id uuid not null,
  attachment_id uuid not null,
  actor_user_id text not null,
  expires_at bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, attachment_id, actor_user_id)
);

do $$ begin
  if not exists(select 1 from pg_constraint where conrelid='attachment_download_links'::regclass
    and conname='attachment_download_links_tenant_owner') then
    alter table attachment_download_links add constraint attachment_download_links_tenant_owner
      foreign key(tenant_id) references tenants(tenant_id) not valid;
  end if;
end $$;

create table if not exists formula_faults (
  tenant_id uuid not null,
  fault_id uuid not null,
  query_no text not null,
  field_code text not null,
  formula_text text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, fault_id)
);

do $$ begin
  if not exists(select 1 from pg_constraint where conrelid='formula_faults'::regclass
    and conname='formula_faults_tenant_owner') then
    alter table formula_faults add constraint formula_faults_tenant_owner
      foreign key(tenant_id) references tenants(tenant_id) not valid;
  end if;
end $$;

create table if not exists labor_rate_orders (
  tenant_id uuid not null,
  employee_id uuid not null,
  order_no text not null,
  rate_cent bigint not null,
  frozen boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, employee_id, order_no)
);

do $$ begin
  if not exists(select 1 from pg_constraint where conrelid='labor_rate_orders'::regclass
    and conname='labor_rate_orders_tenant_owner') then
    alter table labor_rate_orders add constraint labor_rate_orders_tenant_owner
      foreign key(tenant_id) references tenants(tenant_id) not valid;
  end if;
end $$;

create table if not exists labor_rate_versions (
  tenant_id uuid not null,
  employee_id uuid not null,
  order_no text not null,
  rate_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, employee_id, order_no)
);

do $$ begin
  if not exists(select 1 from pg_constraint where conrelid='labor_rate_versions'::regclass
    and conname='labor_rate_versions_tenant_owner') then
    alter table labor_rate_versions add constraint labor_rate_versions_tenant_owner
      foreign key(tenant_id) references tenants(tenant_id) not valid;
  end if;
end $$;

create table if not exists opening_gap_notes (
  tenant_id uuid not null,
  cutoff_date text not null,
  qty_gap bigint not null,
  cost_gap bigint not null,
  explanation text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, cutoff_date)
);

do $$ begin
  if not exists(select 1 from pg_constraint where conrelid='opening_gap_notes'::regclass
    and conname='opening_gap_notes_tenant_owner') then
    alter table opening_gap_notes add constraint opening_gap_notes_tenant_owner
      foreign key(tenant_id) references tenants(tenant_id) not valid;
  end if;
end $$;

create table if not exists shared_balance_checks (
  tenant_id uuid not null,
  warehouse_id uuid not null,
  location_id uuid not null,
  sku_id uuid not null,
  batch_code text not null,
  quality_status text not null,
  opening_qty bigint not null,
  ledger_qty bigint not null,
  stored_qty bigint not null,
  status text not null,
  primary key (tenant_id, warehouse_id, location_id, sku_id, batch_code, quality_status)
);

do $$ begin
  if not exists(select 1 from pg_constraint where conrelid='shared_balance_checks'::regclass
    and conname='shared_balance_checks_tenant_owner') then
    alter table shared_balance_checks add constraint shared_balance_checks_tenant_owner
      foreign key(tenant_id) references tenants(tenant_id) not valid;
  end if;
end $$;

create table if not exists shared_batch_issue_refusals (
  tenant_id uuid not null,
  item_code text not null,
  batch_code text not null,
  request_id text not null,
  reason text not null,
  quality_status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, request_id)
);

do $$ begin
  if not exists(select 1 from pg_constraint where conrelid='shared_batch_issue_refusals'::regclass
    and conname='shared_batch_issue_refusals_tenant_owner') then
    alter table shared_batch_issue_refusals add constraint shared_batch_issue_refusals_tenant_owner
      foreign key(tenant_id) references tenants(tenant_id) not valid;
  end if;
end $$;

create table if not exists shared_issue_refusals (
  tenant_id uuid not null,
  source_line text not null,
  requested_micros bigint not null,
  available_micros bigint not null,
  primary key (tenant_id, source_line)
);

do $$ begin
  if not exists(select 1 from pg_constraint where conrelid='shared_issue_refusals'::regclass
    and conname='shared_issue_refusals_tenant_owner') then
    alter table shared_issue_refusals add constraint shared_issue_refusals_tenant_owner
      foreign key(tenant_id) references tenants(tenant_id) not valid;
  end if;
end $$;

create table if not exists shared_issue_sources (
  tenant_id uuid not null,
  source_line text not null,
  status text not null,
  primary key (tenant_id, source_line)
);

do $$ begin
  if not exists(select 1 from pg_constraint where conrelid='shared_issue_sources'::regclass
    and conname='shared_issue_sources_tenant_owner') then
    alter table shared_issue_sources add constraint shared_issue_sources_tenant_owner
      foreign key(tenant_id) references tenants(tenant_id) not valid;
  end if;
end $$;

create table if not exists shared_paper_codes (
  tenant_id uuid not null,
  code_id uuid not null,
  paper_code text not null,
  item_code text not null,
  status text not null,
  actor_user_id text not null,
  qty_micros bigint not null default 0,
  created_at timestamptz not null default now(),
  primary key (tenant_id, code_id)
);

do $$ begin
  if not exists(select 1 from pg_constraint where conrelid='shared_paper_codes'::regclass
    and conname='shared_paper_codes_tenant_owner') then
    alter table shared_paper_codes add constraint shared_paper_codes_tenant_owner
      foreign key(tenant_id) references tenants(tenant_id) not valid;
  end if;
end $$;

create table if not exists shared_serial_issue_refusals (
  tenant_id uuid not null,
  item_code text not null,
  serial_no text not null,
  request_id text not null,
  reason text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, request_id)
);

do $$ begin
  if not exists(select 1 from pg_constraint where conrelid='shared_serial_issue_refusals'::regclass
    and conname='shared_serial_issue_refusals_tenant_owner') then
    alter table shared_serial_issue_refusals add constraint shared_serial_issue_refusals_tenant_owner
      foreign key(tenant_id) references tenants(tenant_id) not valid;
  end if;
end $$;

create table if not exists shared_transfer_refusals (
  tenant_id uuid not null,
  transfer_id uuid not null,
  requested_micros bigint not null,
  open_micros bigint not null,
  primary key (tenant_id, transfer_id)
);

do $$ begin
  if not exists(select 1 from pg_constraint where conrelid='shared_transfer_refusals'::regclass
    and conname='shared_transfer_refusals_tenant_owner') then
    alter table shared_transfer_refusals add constraint shared_transfer_refusals_tenant_owner
      foreign key(tenant_id) references tenants(tenant_id) not valid;
  end if;
end $$;

create index if not exists formula_faults_query_field on formula_faults(tenant_id,query_no,field_code);

create index if not exists shared_balance_checks_lookup on shared_balance_checks(tenant_id,warehouse_id,sku_id,batch_code);

create index if not exists shared_paper_codes_lookup on shared_paper_codes(tenant_id,paper_code,status);

create index if not exists shared_batch_issue_refusals_lookup on shared_batch_issue_refusals(tenant_id,item_code,batch_code);

create index if not exists shared_serial_issue_refusals_lookup on shared_serial_issue_refusals(tenant_id,item_code,serial_no);
