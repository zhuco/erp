CREATE TABLE IF NOT EXISTS user_shortcuts (
  tenant_id uuid NOT NULL,
  user_id uuid NOT NULL,
  shortcuts_json jsonb NOT NULL DEFAULT '[]'::jsonb,
  updated_at timestamptz NOT NULL DEFAULT now(),
  created_at timestamptz NOT NULL DEFAULT now(),
  PRIMARY KEY (tenant_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_user_shortcuts_tenant_updated
  ON user_shortcuts (tenant_id, updated_at DESC);

COMMENT ON TABLE user_shortcuts IS 'Per-user shortcut toolbar configuration';
COMMENT ON COLUMN user_shortcuts.shortcuts_json IS 'Shortcut entries ordered list';
