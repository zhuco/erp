-- Keep all old command/source rows unchanged. New actions retain verified actor facts.
create table shared_receipt_value_actors (
  tenant_id uuid not null,
  request_id text not null,
  authentication_tenant uuid not null,
  user_id uuid not null check(user_id<>'00000000-0000-0000-0000-000000000000'),
  session_id uuid not null check(session_id<>'00000000-0000-0000-0000-000000000000'),
  warehouse_id uuid not null check(warehouse_id<>'00000000-0000-0000-0000-000000000000'),
  credential_epoch integer not null check(credential_epoch>=0),
  role_id uuid not null,
  permission_code text not null check(permission_code in ('purchase_orders.write','PROCUREMENT.ORDER.WRITE')),
  policy_version integer not null default 1 check(policy_version=1),
  authorized_at timestamptz not null default clock_timestamp(),
  primary key(tenant_id,request_id),
  foreign key(tenant_id,request_id) references shared_receipt_value_commands(tenant_id,request_id),
  check(authentication_tenant=tenant_id)
);
create index shared_receipt_value_actor_history on shared_receipt_value_actors(tenant_id,user_id,authorized_at,request_id);
-- User/session/role rows remain mutable and may be revoked or collected later;
-- the original authority snapshot must survive that legitimate lifecycle.
create trigger receipt_value_actor_original before update or delete on shared_receipt_value_actors
  for each row execute function keep_receipt_value_original();

create function check_receipt_value_authority_commit() returns trigger language plpgsql as $$
begin
  if not exists(
    select 1 from shared_receipt_value_actors a
    join users u on u.tenant_id=a.tenant_id and u.user_id=a.user_id and u.status='ACTIVE'
    join tenants t on t.tenant_id=a.tenant_id and t.credentials_epoch=a.credential_epoch
    join auth_sessions s on s.tenant_id=a.tenant_id and s.user_id=a.user_id and s.session_id=a.session_id
      and s.status='ACTIVE' and s.credentials_epoch=a.credential_epoch and s.expires_at>clock_timestamp()
    join user_roles ur on ur.tenant_id=a.tenant_id and ur.user_id=a.user_id and ur.role_id=a.role_id
    join role_permissions rp on rp.tenant_id=a.tenant_id and rp.role_id=a.role_id and rp.perm_code=a.permission_code
    join user_warehouse_access w on w.tenant_id=a.tenant_id and w.user_id=a.user_id and w.warehouse_id=a.warehouse_id
    join shared_receipt_quality q on q.tenant_id=a.tenant_id and q.quality_id=new.quality_id and q.warehouse_id=a.warehouse_id
    where a.tenant_id=new.tenant_id and a.request_id=new.request_id and a.authentication_tenant=new.tenant_id
  ) then
    raise exception using errcode='PR401',message='receipt current authority expired or differs at commit';
  end if;
  return null;
end $$;
create constraint trigger zz_receipt_value_authority_commit after insert on shared_receipt_value_commands
  deferrable initially deferred for each row execute function check_receipt_value_authority_commit();
