-- 0002: Order detail enrichment fields + sub-order after-sales/refund modeling

-- =======================
-- Orders: enrich snapshot
-- =======================
alter table orders
  add column if not exists goods_amount_cent bigint,
  add column if not exists postage_amount_cent bigint,
  add column if not exists discount_amount_cent bigint,
  add column if not exists platform_discount_cent bigint,
  add column if not exists seller_discount_cent bigint,
  add column if not exists coupon_discount_cent bigint,
  add column if not exists pay_at timestamptz,
  add column if not exists ship_at timestamptz,
  add column if not exists finish_at timestamptz,
  add column if not exists cancel_at timestamptz,
  add column if not exists buyer_message text,
  add column if not exists seller_remark text,
  add column if not exists logistics_company text,
  add column if not exists tracking_no text;

create index if not exists idx_orders_pay_at on orders(tenant_id, pay_at desc);
create index if not exists idx_orders_ship_at on orders(tenant_id, ship_at desc);

-- =============================
-- Order items: sub-order states
-- =============================
alter table order_items
  add column if not exists pay_amount_cent bigint,
  add column if not exists discount_cent bigint,
  add column if not exists refund_state text not null default 'NONE',
  add column if not exists after_sale_state text not null default 'NONE',
  add column if not exists refund_amount_cent bigint,
  add column if not exists after_sale_type text,
  add column if not exists after_sale_id text;

create index if not exists idx_order_items_refund_state on order_items(tenant_id, refund_state);
create index if not exists idx_order_items_after_sale_state on order_items(tenant_id, after_sale_state);
