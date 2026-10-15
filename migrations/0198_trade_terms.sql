-- Cross-border terms freeze currency, rate, tax, and cutoff.
-- A reversal cites the original amount and does not rewrite it.

create table if not exists trade_terms (
  tenant_id uuid not null,
  term_id uuid not null,
  term_no text not null,
  kind text not null,
  cutoff_text text not null,
  currency text not null,
  original_cent bigint not null,
  rate_ppm bigint not null,
  base_currency text not null,
  base_cent bigint not null,
  rate_source text not null,
  tax_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, term_id)
);

create table if not exists trade_reversals (
  tenant_id uuid not null,
  reversal_id uuid not null,
  term_no text not null,
  kind text not null,
  policy_text text not null,
  amount_cent bigint not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, reversal_id)
);
