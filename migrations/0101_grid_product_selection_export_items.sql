-- Durable, job-owned membership for exports accepted from a short-lived selection.
create table if not exists grid_product_selection_exports (
  job_id uuid primary key references export_jobs(job_id) on delete cascade,
  expected_total bigint not null check (expected_total between 0 and 100000)
);

create table if not exists grid_product_selection_export_items (
  job_id uuid not null references grid_product_selection_exports(job_id) on delete cascade,
  ordinal bigint not null check (ordinal >= 0),
  product_id uuid not null,
  data jsonb not null,
  primary key (job_id, ordinal),
  unique (job_id, product_id)
);
