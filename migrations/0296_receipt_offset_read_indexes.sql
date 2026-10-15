-- Receipt-local reconciliation uses actual return ordering and source postings.
-- These indexes support read predicates without changing any stored cost facts.
CREATE INDEX IF NOT EXISTS shared_supplier_returns_receipt_read
    ON shared_supplier_returns (tenant_id, receipt_id, created_at, return_id)
    INCLUDE (qty_micros, cost_cent);

CREATE INDEX IF NOT EXISTS partner_balances_receipt_posting_read
    ON partner_balances (tenant_id, source_id, kind, currency, partner_code, source_type)
    INCLUDE (amount_cent);
