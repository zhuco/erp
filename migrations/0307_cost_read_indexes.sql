-- Bounded original-run and late-adjustment reads must not scan a whole enterprise.
create index if not exists mfg_cost_authorities_run_read
 on mfg_cost_commit_authorities(tenant_id,cost_run_id);
create index if not exists mfg_cost_adjustments_order_read
 on mfg_cost_adjustments(tenant_id,work_order_id,created_at,adjustment_id);
