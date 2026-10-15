-- Managed trade documents extend the existing revision and line facts.
create table trade_sales_documents (
 tenant_id uuid not null,so_id uuid not null,document_no text not null,
 currency text not null,primary key(tenant_id,so_id),unique(tenant_id,document_no),
 check(length(document_no) between 1 and 128),check(currency in('CNY','USD','EUR','HKD'))
);
create table trade_sales_document_revisions (
 tenant_id uuid not null,revision_id uuid not null,so_id uuid not null,request_id text not null,
 partner_id uuid not null,snapshot jsonb not null,
 primary key(tenant_id,revision_id),unique(tenant_id,request_id),
 foreign key(tenant_id,so_id) references trade_sales_documents(tenant_id,so_id),
 foreign key(tenant_id,revision_id,so_id) references shared_sales_revisions(tenant_id,revision_id,so_id),
 foreign key(tenant_id,request_id) references command_receipts(tenant_id,request_id) deferrable initially deferred,
 foreign key(tenant_id,partner_id) references shared_partners(tenant_id,partner_id)
);
create table trade_sales_line_bindings (
 tenant_id uuid not null,revision_id uuid not null,line_no integer not null,sku_id uuid not null,warehouse_id uuid,
 primary key(tenant_id,revision_id,line_no),
 foreign key(tenant_id,revision_id) references trade_sales_document_revisions(tenant_id,revision_id),
 foreign key(tenant_id,revision_id,line_no) references shared_sales_lines(tenant_id,revision_id,line_no),
 foreign key(tenant_id,sku_id) references skus(tenant_id,sku_id),
 foreign key(tenant_id,warehouse_id) references warehouses(tenant_id,warehouse_id)
);
create index trade_sales_document_revision_owner on trade_sales_document_revisions(tenant_id,so_id,revision_id);
create trigger trade_sales_identity_immutable before update or delete on trade_sales_documents
 for each row execute function protect_receipt_disposition_fact();
create trigger trade_sales_header_immutable before update or delete on trade_sales_document_revisions
 for each row execute function protect_receipt_disposition_fact();
create trigger trade_sales_binding_immutable before update or delete on trade_sales_line_bindings
 for each row execute function protect_receipt_disposition_fact();
create function protect_managed_sales_fact() returns trigger language plpgsql as $$
begin
 if exists(select 1 from trade_sales_document_revisions where tenant_id=old.tenant_id and revision_id=old.revision_id)
 then raise exception 'managed sales revision facts are immutable'; end if;
 if tg_op='UPDATE' then return new; end if;
 return old;
end $$;
create trigger managed_sales_revision_immutable before update or delete on shared_sales_revisions
 for each row execute function protect_managed_sales_fact();
create trigger managed_sales_line_immutable before update or delete on shared_sales_lines
 for each row execute function protect_managed_sales_fact();
