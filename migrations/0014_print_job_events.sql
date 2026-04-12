-- 0011: Print job audit events

alter table if exists print_jobs
  add column if not exists template_version int not null default 1,
  add column if not exists requested_by_user_id uuid;

create table if not exists print_job_events (
  event_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  job_id uuid not null references print_jobs(job_id),
  status text not null,
  actor_type text not null,
  actor_user_id uuid,
  error text,
  created_at timestamptz not null default now()
);

create index if not exists idx_print_job_events_job
  on print_job_events(tenant_id, job_id, created_at);
