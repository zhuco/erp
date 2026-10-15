-- A confirmed sales document can open a receivable. A draft cannot.
-- Existing confirmed trade rows stay confirmed. Advances remain a separate kind.

alter table sales_orders add column if not exists status text not null default 'CONFIRMED';
