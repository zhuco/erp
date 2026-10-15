-- Attachment keys include the tenant. A new file with the same name is the next version.

create table if not exists shared_attachments (
  tenant_id uuid not null,
  attachment_id uuid not null,
  object_id text not null,
  file_name text not null,
  version_no integer not null,
  byte_count bigint not null,
  digest text not null,
  storage_key text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, attachment_id)
);

create table if not exists shared_attachment_grants (
  tenant_id uuid not null,
  attachment_id uuid not null,
  actor_user_id text not null,
  revoked boolean not null default false,
  created_at timestamptz not null default now(),
  primary key (tenant_id, attachment_id, actor_user_id)
);

create table if not exists shared_custom_fields (
  tenant_id uuid not null,
  object_id text not null,
  field_name text not null,
  field_type text not null,
  field_value text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, object_id, field_name)
);
