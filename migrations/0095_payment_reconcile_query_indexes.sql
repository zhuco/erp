-- 0095: Improve payment reconcile list query performance for statement filter and diff views

create index if not exists idx_payment_reconcile_jobs_tenant_statement_created
  on payment_reconcile_jobs(tenant_id, statement_id, created_at desc);

create index if not exists idx_payment_reconcile_items_tenant_job_diff_created
  on payment_reconcile_items(tenant_id, job_id, created_at desc)
  where diff_cent <> 0;
