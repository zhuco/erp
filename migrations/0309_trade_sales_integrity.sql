-- No guessed adoption of legacy facts; existing managed snapshots remain immutable.
create or replace function protect_managed_sales_fact() returns trigger language plpgsql as $$
begin
 if tg_op='INSERT' then
  if exists(select 1 from trade_sales_document_revisions where tenant_id=new.tenant_id and revision_id=new.revision_id)
  then raise exception 'managed sales revision facts are immutable'; end if;
  return new;
 end if;
 if exists(select 1 from trade_sales_document_revisions where tenant_id=old.tenant_id and revision_id=old.revision_id)
 then raise exception 'managed sales revision facts are immutable'; end if;
 if tg_op='UPDATE' then return new; end if;return old;
end $$;
create trigger managed_sales_line_insert_immutable before insert on shared_sales_lines
 for each row execute function protect_managed_sales_fact();
create function verify_managed_sales_fact() returns trigger language plpgsql as $$
declare owner_id uuid; expected_amount numeric; actual_amount numeric; line_count bigint; binding_count bigint;
begin
 select so_id into owner_id from shared_sales_revisions where tenant_id=new.tenant_id and revision_id=new.revision_id;
 if not exists(select 1 from trade_sales_documents where tenant_id=new.tenant_id and so_id=owner_id) then return new; end if;
 if not exists(select 1 from trade_sales_document_revisions h join command_receipts c on c.tenant_id=h.tenant_id and c.request_id=h.request_id
  where h.tenant_id=new.tenant_id and h.revision_id=new.revision_id and h.so_id=owner_id and c.status='SUCCEEDED' and c.effect_id=h.revision_id::text)
 then raise exception 'managed sales revision requires its original successful command'; end if;
 select (snapshot->>'total_cent')::numeric into expected_amount from trade_sales_document_revisions where tenant_id=new.tenant_id and revision_id=new.revision_id;
 select count(*),sum(trunc(qty_micros::numeric*price_cent::numeric/1000000)-discount_cent+tax_cent)
 into line_count,actual_amount from shared_sales_lines where tenant_id=new.tenant_id and revision_id=new.revision_id;
 select count(*) into binding_count from trade_sales_line_bindings where tenant_id=new.tenant_id and revision_id=new.revision_id;
 if line_count not between 1 and 1000 or binding_count<>line_count or actual_amount is null or expected_amount is null or actual_amount<>expected_amount
 then raise exception 'managed sales revision quantity/value/bindings are incomplete'; end if;
 return new;
end $$;
create constraint trigger managed_sales_revision_complete after insert on shared_sales_revisions
 deferrable initially deferred for each row execute function verify_managed_sales_fact();
create constraint trigger managed_sales_header_complete after insert on trade_sales_document_revisions
 deferrable initially deferred for each row execute function verify_managed_sales_fact();
create function protect_managed_sales_command() returns trigger language plpgsql as $$
begin
 if exists(select 1 from trade_sales_document_revisions where tenant_id=old.tenant_id and request_id=old.request_id)
 then raise exception 'managed sales original command is immutable'; end if;
 if tg_op='UPDATE' then return new; end if;return old;
end $$;
create trigger managed_sales_command_immutable before update or delete on command_receipts
 for each row execute function protect_managed_sales_command();
