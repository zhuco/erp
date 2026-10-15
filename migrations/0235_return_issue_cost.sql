-- A sales return cites the unit cost stored on the reviewed delivery.
-- The cited cost must match that issue cost. The credit does not invent a second cost.

alter table shared_sales_deliveries add column if not exists unit_cost_cent bigint not null default 0;
