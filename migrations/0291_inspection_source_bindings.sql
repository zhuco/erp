-- New freezes bind actual owned sources. No inference or backfill for legacy freezes.
create unique index inspection_receipt_owner_key on mfg_receipts(tenant_id,receipt_id,work_order_id);
create unique index inspection_sales_revision_owner_key on shared_sales_revisions(tenant_id,revision_id,so_id);
create unique index inspection_template_version_key on shared_inspection_templates(tenant_id,template_id,version_no);

create table shared_inspection_source_bindings (
 tenant_id uuid not null,
 freeze_id uuid not null,
 request_id text not null check(btrim(request_id)<>''),
 payload_hash text not null,
 command_effect_id text generated always as(freeze_id::text) stored,
 source_kind text not null check(source_kind in('WORK_ORDER','PRODUCTION_RECEIPT','SALES_ORDER')),
 source_id uuid not null,
 work_order_id uuid,
 receipt_id uuid,
 sales_revision_id uuid,
 template_id uuid not null,
 version_no integer not null check(version_no>0),
 item_code text not null check(octet_length(item_code) between 1 and 1024),
 batch_code text not null check(octet_length(batch_code) between 1 and 512),
 line_count integer not null check(line_count between 1 and 1000),
 frozen_lines jsonb not null,
 source_snapshot jsonb not null,
 created_at timestamptz not null default clock_timestamp(),
 primary key(tenant_id,freeze_id),
 unique(tenant_id,source_kind,source_id,batch_code),
 foreign key(tenant_id,freeze_id) references shared_inspection_freezes(tenant_id,freeze_id),
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id)
  deferrable initially deferred,
 foreign key(tenant_id,work_order_id) references mfg_work_orders(tenant_id,work_order_id),
 foreign key(tenant_id,receipt_id,work_order_id) references mfg_receipts(tenant_id,receipt_id,work_order_id),
 foreign key(tenant_id,sales_revision_id,source_id) references shared_sales_revisions(tenant_id,revision_id,so_id),
 foreign key(tenant_id,template_id,version_no) references shared_inspection_templates(tenant_id,template_id,version_no),
 check((source_kind='WORK_ORDER' and source_id=work_order_id and work_order_id is not null and receipt_id is null and sales_revision_id is null)
  or(source_kind='PRODUCTION_RECEIPT' and source_id=receipt_id and receipt_id is not null and work_order_id is not null and sales_revision_id is null)
  or(source_kind='SALES_ORDER' and sales_revision_id is not null and work_order_id is null and receipt_id is null)),
 check(coalesce(jsonb_typeof(frozen_lines)='array' and jsonb_array_length(frozen_lines)=line_count,false)),
 check(coalesce(jsonb_typeof(source_snapshot)='object' and source_snapshot->>'tenant_id'=tenant_id::text
  and source_snapshot->>'source_kind'=source_kind and source_snapshot->>'source_id'=source_id::text
  and source_snapshot->>'item_code'=item_code and source_snapshot->>'batch_code'=batch_code,false))
);
create index inspection_binding_work_order on shared_inspection_source_bindings(tenant_id,work_order_id);
create index inspection_binding_receipt on shared_inspection_source_bindings(tenant_id,receipt_id);
create index inspection_binding_revision on shared_inspection_source_bindings(tenant_id,sales_revision_id);
create index inspection_binding_template on shared_inspection_source_bindings(tenant_id,template_id,version_no);
create index inspection_binding_command on shared_inspection_source_bindings(tenant_id,request_id);
create index inspection_freeze_source_batch on shared_inspection_freezes(tenant_id,source_id,batch_code,freeze_id);
create index inspection_quality_freeze_item on mfg_quality_items(tenant_id,freeze_id,item_code);

-- New quality approvals retain their original proof without changing old review rows.
create table mfg_quality_release_proofs (
 tenant_id uuid not null,
 release_id uuid not null,
 receipt_id uuid not null,
 work_order_id uuid not null,
 request_id text not null check(btrim(request_id)<>''),
 payload_hash text not null,
 command_effect_id text generated always as(release_id::text) stored,
 snapshot jsonb not null,
 created_at timestamptz not null default clock_timestamp(),
 primary key(tenant_id,release_id),
 unique(tenant_id,receipt_id),
 unique(tenant_id,request_id),
 foreign key(tenant_id,release_id) references mfg_quality_releases(tenant_id,release_id),
 foreign key(tenant_id,receipt_id,work_order_id) references mfg_receipts(tenant_id,receipt_id,work_order_id),
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id)
  deferrable initially deferred,
 check(coalesce(jsonb_typeof(snapshot)='object' and snapshot->>'tenant_id'=tenant_id::text
  and snapshot->>'release_id'=release_id::text and snapshot->>'receipt_id'=receipt_id::text
  and snapshot->>'work_order_id'=work_order_id::text and jsonb_typeof(snapshot->'quality')='object',false))
);
create index inspection_review_command on mfg_quality_release_proofs(tenant_id,request_id);
create index inspection_review_owner on mfg_quality_release_proofs(tenant_id,receipt_id,work_order_id);
create function guard_inspection_review_proof() returns trigger language plpgsql as $$
begin
 if tg_table_name='mfg_quality_release_proofs'
  or exists(select 1 from mfg_quality_release_proofs where tenant_id=old.tenant_id and release_id=old.release_id) then
  raise exception 'original quality approval and proof are immutable';
 end if;
 if tg_op='DELETE' then return old; end if;
 return new;
end $$;
create trigger inspection_review_proof_immutable before update or delete on mfg_quality_release_proofs
 for each row execute function guard_inspection_review_proof();
create trigger inspection_review_parent_immutable before update or delete on mfg_quality_releases
 for each row execute function guard_inspection_review_proof();

-- Serialize insertion against sealing the parent, then validate the whole copy once.
-- Rechecking the whole JSON array for each of 1000 inserted lines is unnecessary.
create function lock_inspection_copy_parent() returns trigger language plpgsql as $$
begin
 perform 1 from shared_inspection_freezes where tenant_id=new.tenant_id and freeze_id=new.freeze_id for update;
 if tg_table_name='shared_inspection_freeze_lines'
  and exists(select 1 from shared_inspection_source_bindings where tenant_id=new.tenant_id and freeze_id=new.freeze_id) then
  raise exception 'owned frozen inspection copy cannot gain extra lines';
 end if;
 return new;
end $$;
create trigger inspection_binding_parent_lock before insert on shared_inspection_source_bindings
 for each row execute function lock_inspection_copy_parent();
create trigger inspection_line_parent_lock before insert on shared_inspection_freeze_lines
 for each row execute function lock_inspection_copy_parent();

create function guard_inspection_bound_copy() returns trigger language plpgsql as $$
begin
 if tg_table_name='shared_inspection_source_bindings'
  or exists(select 1 from shared_inspection_source_bindings b where b.tenant_id=old.tenant_id and b.freeze_id=old.freeze_id) then
  raise exception 'owned frozen inspection source or copy is immutable';
 end if;
 if tg_op='DELETE' then return old; end if;
 return new;
end $$;
create trigger inspection_binding_immutable before update or delete on shared_inspection_source_bindings
 for each row execute function guard_inspection_bound_copy();
create trigger inspection_bound_header_immutable before update or delete on shared_inspection_freezes
 for each row execute function guard_inspection_bound_copy();
create trigger inspection_bound_line_immutable before update or delete on shared_inspection_freeze_lines
 for each row execute function guard_inspection_bound_copy();
create trigger inspection_bound_reading_immutable before update or delete on mfg_quality_items
 for each row when(old.freeze_id is not null) execute function guard_inspection_bound_copy();

create function guard_inspection_bound_command() returns trigger language plpgsql as $$
begin
 if exists(select 1 from shared_inspection_source_bindings where tenant_id=old.tenant_id and request_id=old.request_id)
  or exists(select 1 from mfg_quality_release_proofs where tenant_id=old.tenant_id and request_id=old.request_id) then
  raise exception 'original inspection freeze command is immutable';
 end if;
 if tg_op='DELETE' then return old; end if;
 return new;
end $$;
create trigger inspection_bound_command_immutable before update or delete on command_receipts
 for each row execute function guard_inspection_bound_command();

create function verify_inspection_bound_copy() returns trigger language plpgsql as $$
declare binding shared_inspection_source_bindings%rowtype; actual jsonb;
begin
 select * into binding from shared_inspection_source_bindings where tenant_id=new.tenant_id and freeze_id=new.freeze_id;
 if not found then return new; end if;
 if not exists(select 1 from shared_inspection_freezes f where f.tenant_id=binding.tenant_id and f.freeze_id=binding.freeze_id
  and f.template_id=binding.template_id and f.version_no=binding.version_no and f.source_kind=binding.source_kind
  and f.source_id=binding.source_id and f.item_code=binding.item_code and f.batch_code=binding.batch_code) then
  raise exception 'inspection frozen header and owned source differ';
 end if;
 if tg_table_name='shared_inspection_source_bindings' then
  select jsonb_agg(jsonb_build_object('item_code',item_code,'required',required,'min_micros',min_micros,
   'max_micros',max_micros,'sample_count',sample_count) order by item_code) into actual
  from shared_inspection_freeze_lines where tenant_id=binding.tenant_id and freeze_id=binding.freeze_id;
  if actual is distinct from binding.frozen_lines then
   raise exception 'inspection frozen copy is incomplete or differs from its original snapshot';
  end if;
 end if;
 if tg_table_name='mfg_quality_items' then
  if new.source_kind<>binding.source_kind or new.source_id is distinct from binding.source_id
   or new.work_order_id is distinct from binding.work_order_id
   or new.batch_code<>binding.batch_code or not exists(select 1 from shared_inspection_freeze_lines l
    where l.tenant_id=new.tenant_id and l.freeze_id=new.freeze_id and l.item_code=new.item_code
    and l.required=new.required and l.min_micros=new.min_micros and l.max_micros=new.max_micros) then
   raise exception 'inspection reading does not own the frozen source and range';
  end if;
 end if;
 return new;
end $$;
create constraint trigger inspection_binding_complete after insert on shared_inspection_source_bindings
 deferrable initially deferred for each row execute function verify_inspection_bound_copy();
create constraint trigger inspection_bound_reading_complete after insert on mfg_quality_items
 deferrable initially deferred for each row when(new.freeze_id is not null) execute function verify_inspection_bound_copy();
