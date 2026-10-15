-- Releasable clients are the Windows desktop binary and the WASM package.
-- Offline actions may keep a draft. They do not write inventory or claim a save.

create table if not exists client_artifacts (
  tenant_id uuid not null,
  artifact_id uuid not null,
  platform text not null,
  artifact_name text not null,
  version_text text not null,
  server_range text not null,
  entry_text text not null,
  config_text text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, artifact_id)
);

create table if not exists platform_adapters (
  tenant_id uuid not null,
  platform text not null,
  adapter text not null,
  present boolean not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, platform, adapter)
);

create table if not exists offline_client_actions (
  tenant_id uuid not null,
  action_id uuid not null,
  platform text not null,
  online boolean not null,
  action text not null,
  decision text not null,
  draft_text text not null default '',
  created_at timestamptz not null default now(),
  primary key (tenant_id, action_id)
);
