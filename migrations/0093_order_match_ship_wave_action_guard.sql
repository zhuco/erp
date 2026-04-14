-- 0093: Hard guard for order/match/ship/wave persistence chains.
-- This migration is idempotent and keeps existing environments forward compatible.

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'ck_order_action_log_action_code_not_blank'
  ) THEN
    ALTER TABLE order_action_log
      ADD CONSTRAINT ck_order_action_log_action_code_not_blank
      CHECK (btrim(action_code) <> '') NOT VALID;
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'ck_order_action_log_idempotency_key_not_blank'
  ) THEN
    ALTER TABLE order_action_log
      ADD CONSTRAINT ck_order_action_log_idempotency_key_not_blank
      CHECK (btrim(idempotency_key) <> '') NOT VALID;
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'ck_order_action_log_status_not_blank'
  ) THEN
    ALTER TABLE order_action_log
      ADD CONSTRAINT ck_order_action_log_status_not_blank
      CHECK (btrim(status) <> '') NOT VALID;
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'ck_order_status_history_to_status_not_blank'
  ) THEN
    ALTER TABLE order_status_history
      ADD CONSTRAINT ck_order_status_history_to_status_not_blank
      CHECK (btrim(to_status) <> '') NOT VALID;
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'ck_order_status_history_action_code_not_blank'
  ) THEN
    ALTER TABLE order_status_history
      ADD CONSTRAINT ck_order_status_history_action_code_not_blank
      CHECK (action_code IS NULL OR btrim(action_code) <> '') NOT VALID;
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'ck_operation_logs_actor_type_not_blank'
  ) THEN
    ALTER TABLE operation_logs
      ADD CONSTRAINT ck_operation_logs_actor_type_not_blank
      CHECK (btrim(actor_type) <> '') NOT VALID;
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'ck_operation_logs_action_not_blank'
  ) THEN
    ALTER TABLE operation_logs
      ADD CONSTRAINT ck_operation_logs_action_not_blank
      CHECK (btrim(action) <> '') NOT VALID;
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'ck_audit_logs_action_not_blank'
  ) THEN
    ALTER TABLE audit_logs
      ADD CONSTRAINT ck_audit_logs_action_not_blank
      CHECK (btrim(action) <> '') NOT VALID;
  END IF;
END $$;

DO $$
BEGIN
  IF NOT EXISTS (
    SELECT 1 FROM pg_constraint
    WHERE conname = 'ck_audit_logs_entity_type_not_blank'
  ) THEN
    ALTER TABLE audit_logs
      ADD CONSTRAINT ck_audit_logs_entity_type_not_blank
      CHECK (btrim(entity_type) <> '') NOT VALID;
  END IF;
END $$;

create index if not exists idx_operation_logs_action_created
  on operation_logs(tenant_id, action, created_at desc, log_id desc);

create index if not exists idx_audit_logs_action_created
  on audit_logs(tenant_id, action, created_at desc, audit_id desc);
