-- Append real measured outcomes and reviews. Do not alter or guess historical readings.
create unique index inspection_reading_actor_owner_key on users(tenant_id,user_id);
create table shared_inspection_reading_events (
 tenant_id uuid not null,
 item_id uuid not null,
 freeze_id uuid not null,
 item_code text not null check(octet_length(item_code) between 1 and 1024),
 attempt_no integer not null check(attempt_no between 1 and 20),
 previous_item_id uuid,
 review_required boolean not null,
 verdict text not null check(verdict in('PASSED','FAILED','MISSING')),
 declared_reviewer text not null check(octet_length(declared_reviewer) between 1 and 512 and btrim(declared_reviewer)<>''),
 actor_source text not null check(actor_source in('USER','SYSTEM')),
 actor_user_id uuid,
 reason text not null check(octet_length(reason)<=2048),
 request_id text not null,
 payload_hash text not null,
 command_effect_id text generated always as(item_id::text) stored,
 snapshot jsonb not null,
 created_at timestamptz not null default clock_timestamp(),
 primary key(tenant_id,item_id),
 unique(tenant_id,freeze_id,item_code,attempt_no),
 unique(tenant_id,previous_item_id),
 unique(tenant_id,request_id),
 foreign key(tenant_id,item_id) references mfg_quality_items(tenant_id,item_id),
 foreign key(tenant_id,freeze_id,item_code) references shared_inspection_freeze_lines(tenant_id,freeze_id,item_code),
 foreign key(tenant_id,freeze_id) references shared_inspection_source_bindings(tenant_id,freeze_id),
 foreign key(tenant_id,previous_item_id) references shared_inspection_reading_events(tenant_id,item_id),
 foreign key(tenant_id,actor_user_id) references users(tenant_id,user_id),
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id) deferrable initially deferred,
 check((actor_source='USER' and actor_user_id is not null) or(actor_source='SYSTEM' and actor_user_id is null)),
 check((attempt_no=1 and previous_item_id is null and review_required=false and reason='')
    or(attempt_no>1 and previous_item_id is not null and review_required=true and btrim(reason)<>'')),
 check(coalesce(jsonb_typeof(snapshot)='object' and snapshot->>'tenant_id'=tenant_id::text
   and snapshot->>'item_id'=item_id::text and snapshot->>'freeze_id'=freeze_id::text
   and snapshot->>'verdict'=verdict and snapshot->>'item_code'=item_code,false))
);
create table shared_inspection_reading_reviews (
 tenant_id uuid not null,
 review_id uuid not null,
 item_id uuid not null,
 reviewer_user_id uuid not null,
 decision text not null check(decision in('APPROVE','REJECT')),
 reason text not null check(octet_length(reason)<=2048 and(decision<>'REJECT' or btrim(reason)<>'')),
 request_id text not null,
 payload_hash text not null,
 command_effect_id text generated always as(review_id::text) stored,
 snapshot jsonb not null,
 created_at timestamptz not null default clock_timestamp(),
 primary key(tenant_id,review_id),
 unique(tenant_id,item_id),
 unique(tenant_id,request_id),
 foreign key(tenant_id,item_id) references shared_inspection_reading_events(tenant_id,item_id),
 foreign key(tenant_id,reviewer_user_id) references users(tenant_id,user_id),
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id) deferrable initially deferred,
 check(coalesce(jsonb_typeof(snapshot)='object' and snapshot->>'tenant_id'=tenant_id::text
   and snapshot->>'review_id'=review_id::text and snapshot->>'item_id'=item_id::text
   and snapshot->>'reviewer_user_id'=reviewer_user_id::text and snapshot->>'decision'=decision,false))
);
create index inspection_outcome_original_command on shared_inspection_reading_events(tenant_id,request_id);
create index inspection_outcome_previous on shared_inspection_reading_events(tenant_id,previous_item_id);
create index inspection_outcome_actor on shared_inspection_reading_events(tenant_id,actor_user_id);
create index inspection_reading_review_actor on shared_inspection_reading_reviews(tenant_id,reviewer_user_id);
create index inspection_reading_review_command on shared_inspection_reading_reviews(tenant_id,request_id);

create function guard_inspection_outcome() returns trigger language plpgsql as $$
begin
 raise exception 'original inspection reading provenance and review are immutable';
end $$;
create trigger inspection_outcome_immutable before update or delete on shared_inspection_reading_events
 for each row execute function guard_inspection_outcome();
create trigger inspection_reading_review_immutable before update or delete on shared_inspection_reading_reviews
 for each row execute function guard_inspection_outcome();
create function guard_inspection_outcome_command() returns trigger language plpgsql as $$
begin
 if exists(select 1 from shared_inspection_reading_events where tenant_id=old.tenant_id and request_id=old.request_id)
  or exists(select 1 from shared_inspection_reading_reviews where tenant_id=old.tenant_id and request_id=old.request_id) then
  raise exception 'original inspection reading or review command is immutable';
 end if;
 if tg_op='DELETE' then return old; end if;
 return new;
end $$;
create trigger inspection_outcome_command_immutable before update or delete on command_receipts
 for each row execute function guard_inspection_outcome_command();
create constraint trigger inspection_outcome_original_payload after insert on shared_inspection_reading_events
 deferrable initially deferred for each row execute function verify_inspection_original_command();
create constraint trigger inspection_reading_review_original_payload after insert on shared_inspection_reading_reviews
 deferrable initially deferred for each row execute function verify_inspection_original_command();

create function verify_inspection_outcome() returns trigger language plpgsql as $$
declare q mfg_quality_items%rowtype; p shared_inspection_reading_events%rowtype; expected text;
begin
 select * into q from mfg_quality_items where tenant_id=new.tenant_id and item_id=new.item_id for share;
 if not found or q.freeze_id is distinct from new.freeze_id or q.item_code<>new.item_code then
  raise exception 'inspection outcome must own its actual frozen reading';
 end if;
 expected:=case when not q.reading_present then 'MISSING'
   when q.reading_micros between q.min_micros and q.max_micros then 'PASSED' else 'FAILED' end;
 if expected<>new.verdict or q.min_micros>q.max_micros
  or(q.reading_present and btrim(q.inspector)='')
  or q.passed<>(case when not q.reading_present then not q.required else expected='PASSED' end)
  or not(new.snapshot @> jsonb_build_object(
    'reading_micros',case when q.reading_present then q.reading_micros::text else '' end,
    'reading_present',q.reading_present,'passed',q.passed,'required',q.required,
    'source_kind',q.source_kind,'source_id',q.source_id,'batch_code',q.batch_code,
    'min_micros',q.min_micros::text,'max_micros',q.max_micros::text,
    'inspector',q.inspector,'declared_reviewer',new.declared_reviewer,
    'actor_source',new.actor_source,'actor_user_id',new.actor_user_id,
    'attempt_no',new.attempt_no,'previous_item_id',new.previous_item_id,
    'reason',new.reason,'review_required',new.review_required)) then
  raise exception 'inspection outcome differs from original measured value or actor';
 end if;
 if new.attempt_no>1 then
  select * into p from shared_inspection_reading_events where tenant_id=new.tenant_id and item_id=new.previous_item_id for share;
  if not found or p.freeze_id<>new.freeze_id or p.item_code<>new.item_code or p.attempt_no+1<>new.attempt_no then
   raise exception 'reinspection must append to the same original line';
  end if;
  if p.verdict='PASSED' and not exists(select 1 from shared_inspection_reading_reviews r
    where r.tenant_id=p.tenant_id and r.item_id=p.item_id and r.decision='REJECT') then
   raise exception 'a passed reading cannot be replaced without an actual rejection';
  end if;
 end if;
 return new;
end $$;
create trigger inspection_outcome_measured_value before insert on shared_inspection_reading_events
 for each row execute function verify_inspection_outcome();

create function verify_inspection_reading_review() returns trigger language plpgsql as $$
declare e shared_inspection_reading_events%rowtype;
begin
 select * into e from shared_inspection_reading_events where tenant_id=new.tenant_id and item_id=new.item_id for share;
 if not found or exists(select 1 from shared_inspection_reading_events n
   where n.tenant_id=e.tenant_id and n.freeze_id=e.freeze_id and n.item_code=e.item_code and n.attempt_no>e.attempt_no)
  or(new.decision='APPROVE' and e.verdict<>'PASSED')
  or new.snapshot->'reading' is distinct from e.snapshot then
  raise exception 'review must cite the latest original reading; a measured failure cannot be approved';
 end if;
 return new;
end $$;
create trigger inspection_reading_review_latest before insert on shared_inspection_reading_reviews
 for each row execute function verify_inspection_reading_review();
