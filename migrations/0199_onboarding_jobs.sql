-- One verification opens one enterprise. A failed step stays resumable.
-- Completing initialization does not consume the verification a second time.

create table if not exists onboarding_jobs (
  tenant_id uuid not null,
  job_id uuid not null,
  identity_text text not null,
  verification_token text not null,
  company_name text not null,
  mode_code text not null,
  timezone_name text not null,
  currency text not null,
  status text not null,
  expires_at timestamptz not null,
  opened_at timestamptz not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, job_id),
  unique (tenant_id, verification_token)
);

create table if not exists onboarding_steps (
  tenant_id uuid not null,
  step_id uuid not null,
  job_id uuid not null,
  step_code text not null,
  status text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, step_id)
);
