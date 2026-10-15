-- Page instances stay with one enterprise, one user, and one page.
-- Hiding a panel does not drop the draft. Closing a page does not cancel an accepted write.

create table if not exists page_instances (
  tenant_id uuid not null,
  page_row_id uuid not null,
  user_id text not null,
  page_code text not null,
  instance_code text not null,
  service_code text not null,
  theme_code text not null,
  left_hidden boolean not null,
  right_hidden boolean not null,
  subtable_hidden boolean not null,
  query_text text not null,
  draft_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, page_row_id),
  unique (tenant_id, user_id, page_code, instance_code)
);

create table if not exists page_close_tasks (
  tenant_id uuid not null,
  task_id uuid not null,
  page_code text not null,
  instance_code text not null,
  task_kind text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, task_id)
);
