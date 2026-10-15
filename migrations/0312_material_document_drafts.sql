-- Real, versioned material documents. Drafts do not post stock or value.
create table mfg_material_documents (
 tenant_id uuid not null references tenants(tenant_id), document_id uuid not null,
 work_order_id uuid not null, version_no integer not null check(version_no>0),
 source_snapshot jsonb not null, status text not null default 'DRAFT' check(status in('DRAFT','POSTED')),
 posted_request_id text, posted_snapshot jsonb,
 created_at timestamptz not null default now(), updated_at timestamptz not null default now(),
 primary key(tenant_id,document_id),
 foreign key(tenant_id,work_order_id) references mfg_work_orders(tenant_id,work_order_id),
 foreign key(tenant_id,posted_request_id) references command_receipts(tenant_id,request_id) deferrable initially deferred,
 check((status='DRAFT' and posted_request_id is null and posted_snapshot is null)
    or (status='POSTED' and posted_request_id is not null and posted_snapshot is not null))
);
create table mfg_material_document_revisions (
 tenant_id uuid not null, document_id uuid not null, version_no integer not null,
 revision_id uuid not null, request_id text not null, actor_user_id uuid not null,
 snapshot jsonb not null, created_at timestamptz not null default now(),
 primary key(tenant_id,document_id,version_no), unique(tenant_id,revision_id), unique(tenant_id,request_id),
 foreign key(tenant_id,document_id) references mfg_material_documents(tenant_id,document_id),
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id) deferrable initially deferred,
 foreign key(tenant_id,actor_user_id) references users(tenant_id,user_id)
);
alter table mfg_material_documents add constraint mfg_material_document_current_revision
 foreign key(tenant_id,document_id,version_no) references mfg_material_document_revisions(tenant_id,document_id,version_no)
 deferrable initially deferred;
create table mfg_material_document_lines (
 tenant_id uuid not null, document_id uuid not null, line_id uuid not null,
 ordinal integer not null check(ordinal between 1 and 100),
 kind text not null check(kind in('ISSUE','SUPPLEMENT','RETURN','CONSUME')),
 warehouse_id uuid not null, sku_id uuid not null, batch_id uuid, original_issue_id uuid,
 unit_code text not null check(octet_length(unit_code) between 1 and 32),
 qty_micros bigint not null check(qty_micros>0), reason text check(octet_length(reason)<=4096),
 primary key(tenant_id,document_id,line_id), unique(tenant_id,document_id,ordinal),
 foreign key(tenant_id,document_id) references mfg_material_documents(tenant_id,document_id),
 foreign key(tenant_id,warehouse_id) references warehouses(tenant_id,warehouse_id),
 foreign key(tenant_id,sku_id) references skus(tenant_id,sku_id),
 foreign key(tenant_id,original_issue_id) references mfg_material_moves(tenant_id,move_id),
 check((kind in('RETURN','CONSUME'))=(original_issue_id is not null)),
 check(kind<>'SUPPLEMENT' or (reason is not null and length(btrim(reason))>0))
);
create index mfg_material_document_order on mfg_material_documents(tenant_id,work_order_id,document_id);
create trigger mfg_material_revision_immutable before update or delete on mfg_material_document_revisions
 for each row execute function protect_receipt_disposition_fact();
create function protect_material_document() returns trigger language plpgsql as $$
begin
 if TG_OP='DELETE' or OLD.status='POSTED' or NEW.tenant_id<>OLD.tenant_id
    or NEW.document_id<>OLD.document_id or NEW.work_order_id<>OLD.work_order_id then
  raise exception 'material document identity or posted facts are immutable' using errcode='55000';
 end if;
 return NEW;
end $$;
create trigger mfg_material_document_identity before update or delete on mfg_material_documents
 for each row execute function protect_material_document();
create function protect_material_document_line() returns trigger language plpgsql as $$
declare owner_tenant uuid; owner_document uuid; state text;
begin
 if TG_OP='INSERT' then owner_tenant:=NEW.tenant_id; owner_document:=NEW.document_id;
 else owner_tenant:=OLD.tenant_id; owner_document:=OLD.document_id; end if;
 select status into state from mfg_material_documents where tenant_id=owner_tenant and document_id=owner_document for share;
 if state='POSTED' then
  raise exception 'posted material lines are immutable' using errcode='55000';
 end if;
 if TG_OP='UPDATE' and (NEW.tenant_id<>OLD.tenant_id or NEW.document_id<>OLD.document_id or NEW.line_id<>OLD.line_id) then
  raise exception 'material line identity is immutable' using errcode='55000';
 end if;
 if TG_OP='DELETE' then return OLD; end if;
 return NEW;
end $$;
create trigger mfg_material_document_line_owner before insert or update or delete on mfg_material_document_lines
 for each row execute function protect_material_document_line();
