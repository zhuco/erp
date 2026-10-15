-- Keep original report outcomes; never invent outcomes for historical reports.
create table mfg_operation_report_outcomes (
 tenant_id uuid not null, request_id text not null, work_order_id uuid not null,
 report_id uuid not null, actor_user_id uuid not null, payload_hash text not null,
 snapshot jsonb not null check(jsonb_typeof(snapshot)='object'),
 created_at timestamptz not null default now(),
 primary key(tenant_id,request_id), unique(tenant_id,report_id),
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id),
 foreign key(tenant_id,work_order_id) references mfg_work_orders(tenant_id,work_order_id),
 foreign key(tenant_id,report_id) references mfg_reports(tenant_id,report_id),
 foreign key(tenant_id,actor_user_id) references users(tenant_id,user_id)
);
create trigger mfg_operation_report_outcome_immutable before update or delete on mfg_operation_report_outcomes
 for each row execute function protect_receipt_disposition_fact();
