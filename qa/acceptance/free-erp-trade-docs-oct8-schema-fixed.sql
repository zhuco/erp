\set ON_ERROR_STOP on
begin;
do $$ begin if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test' or inet_server_port()<>15438 or inet_server_addr()<>inet '127.0.0.1' then raise exception 'protected QA identity mismatch'; end if; end $$;
\ir G:/erp/migrations/0308_trade_sales_documents.sql
commit;