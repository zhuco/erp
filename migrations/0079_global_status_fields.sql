-- 0079: Global status fields (spec section 1)

-- Orders
alter table orders
  add column if not exists order_status text,
  add column if not exists fulfillment_status text,
  add column if not exists exception_status text;

-- Outbound docs (WMS)
alter table outbound_docs
  add column if not exists outbound_status text,
  add column if not exists pick_status text,
  add column if not exists verify_status text,
  add column if not exists pack_status text,
  add column if not exists ship_status text,
  add column if not exists picked_at timestamptz,
  add column if not exists packed_at timestamptz,
  add column if not exists shipped_at timestamptz;

-- Inbound docs (WMS)
alter table inbound_docs
  add column if not exists inbound_status text,
  add column if not exists receipt_status text,
  add column if not exists inbound_qc_status text,
  add column if not exists putaway_status text;

-- After-sales
alter table after_sales
  add column if not exists after_sale_status text,
  add column if not exists compensation_status text,
  add column if not exists evidence_status text;

alter table return_receipts
  add column if not exists return_status text;

alter table refund_docs
  add column if not exists refund_status text;

-- Procurement / AP
alter table purchase_orders
  add column if not exists purchase_order_status text;

alter table purchase_receipts
  add column if not exists receiving_status text;

alter table ap_docs
  add column if not exists settlement_status text;

-- Shipments
alter table shipments
  add column if not exists ship_status text,
  add column if not exists last_push_error text;
