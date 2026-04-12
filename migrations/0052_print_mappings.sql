-- 0049: Print mapping settings

create table if not exists print_mappings (
  mapping_id uuid primary key default gen_random_uuid(),
  tenant_id uuid not null references tenants(tenant_id),
  doc_type text not null,
  template_id uuid not null references print_templates(template_id),
  device_fingerprint text,
  printer_name text,
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now(),
  unique (tenant_id, doc_type)
);

create index if not exists idx_print_mappings_tenant_doc
  on print_mappings(tenant_id, doc_type);
