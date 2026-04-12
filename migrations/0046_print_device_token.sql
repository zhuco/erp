-- 0038: Print device token + job device assignment

alter table if exists print_devices
  add column if not exists device_token text;

create unique index if not exists idx_print_devices_token
  on print_devices(device_token);

alter table if exists print_jobs
  add column if not exists device_fingerprint text;

create index if not exists idx_print_jobs_device_status
  on print_jobs(tenant_id, device_fingerprint, status, created_at);
