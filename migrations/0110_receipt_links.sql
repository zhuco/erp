-- 0110: link a production receipt to its report, batch, inspection, and quantity ledger.
-- A completion flag does not zero WIP. Held goods stay unavailable.

alter table mfg_receipts add column if not exists report_id uuid;
alter table mfg_receipts add column if not exists batch_code text;
alter table mfg_receipts add column if not exists inspection_id uuid;
alter table mfg_receipts add column if not exists ledger_id uuid;

alter table mfg_qty_ledgers add column if not exists source_receipt_id uuid;

create table if not exists mfg_wip_balances (
  tenant_id uuid not null references tenants(tenant_id),
  work_order_id uuid not null,
  wip_micros bigint not null,
  completed boolean not null default false,
  primary key (tenant_id, work_order_id)
);
