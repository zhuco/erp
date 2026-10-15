-- Formal password recovery must exist after installation, not be created by a test.
-- Keep any legacy rows intact. The owner constraint applies to new writes;
-- validating existing imported tokens is a separate upgrade acceptance step.
create unique index if not exists users_password_recovery_owner_key
  on users(tenant_id, user_id);

create table if not exists password_reset_tokens (
  tenant_id uuid not null,
  user_id uuid not null,
  token_hash text not null,
  expires_at timestamptz not null,
  used_at timestamptz,
  created_at timestamptz not null default now(),
  primary key (tenant_id, token_hash)
);

do $$ begin
  if not exists(select 1 from pg_constraint
    where conrelid='password_reset_tokens'::regclass
      and conname='password_reset_token_user_owner') then
    alter table password_reset_tokens
      add constraint password_reset_token_user_owner
      foreign key(tenant_id,user_id) references users(tenant_id,user_id) not valid;
  end if;
end $$;

create index if not exists password_reset_tokens_unused_owner
  on password_reset_tokens(tenant_id,user_id,expires_at)
  where used_at is null;
