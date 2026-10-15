-- An operation route is versioned. A work order freezes the sequence.
-- A later route does not rewrite the frozen order. Display order is not sequence.

create table if not exists shared_operation_routes (
  tenant_id uuid not null,
  route_id uuid not null,
  product_code text not null,
  version_no integer not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, route_id),
  unique (tenant_id, product_code, version_no)
);

create table if not exists shared_operation_steps (
  tenant_id uuid not null,
  route_id uuid not null,
  operation_code text not null,
  sequence_no integer not null,
  predecessor_code text not null default '',
  work_center text not null,
  setup_minutes bigint not null,
  minutes_per bigint not null,
  inspection_required boolean not null,
  report_field text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, route_id, operation_code)
);

create table if not exists shared_work_order_routes (
  tenant_id uuid not null,
  work_order_id uuid not null,
  route_id uuid not null,
  version_no integer not null,
  operation_code text not null,
  sequence_no integer not null,
  predecessor_code text not null,
  work_center text not null,
  setup_minutes bigint not null,
  minutes_per bigint not null,
  inspection_required boolean not null,
  report_field text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, work_order_id, operation_code)
);
