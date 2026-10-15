-- Command lookup keeps object, version, line result, and a recoverable error.
-- A lost response or a revoked session does not delete a committed receipt.

create table if not exists command_objects (
  tenant_id uuid not null,
  request_id text not null,
  object_type text not null,
  object_id text not null,
  version_no bigint not null,
  line_no integer not null,
  line_status text not null,
  recoverable boolean not null,
  error_text text not null default '',
  accepted_at timestamptz not null,
  completed_at timestamptz,
  primary key (tenant_id, request_id, line_no)
);
