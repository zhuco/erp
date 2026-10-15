-- Forecast rows stay beside sales, stock, and production facts.
-- A disabled recipient does not receive a subscribed report.

create table if not exists report_subscriptions (
  tenant_id uuid not null,
  subscription_id uuid not null,
  owner_user_id text not null,
  recipient_user_id text not null,
  report_scope text not null,
  active boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, subscription_id)
);

create table if not exists report_deliveries (
  tenant_id uuid not null,
  delivery_id uuid not null,
  subscription_id uuid not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, delivery_id)
);

create table if not exists forecast_evaluations (
  tenant_id uuid not null,
  forecast_id uuid not null,
  model_version text not null,
  cutoff_text text not null,
  holdout_from text not null,
  holdout_to text not null,
  predicted bigint not null,
  actual bigint not null,
  naive bigint not null,
  error_abs bigint not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, forecast_id)
);
