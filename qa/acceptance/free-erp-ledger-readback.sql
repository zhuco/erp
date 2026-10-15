begin read only;
do $$ begin
 if current_database()<>'erp_shared_contract_test' or current_user<>'erp_grid_test'
  or inet_server_addr()<>'127.0.0.1'::inet or inet_server_port()<>15438 then
  raise exception 'Protected QA identity required';
 end if;
end $$;
with selected(test_case,tenant_id) as (values
('item_cost::cost_full_issue_preserves_two_cent_value_and_zero_unit_rebuild','bb170ac5-09c6-4e56-9e23-5edbce345910'::uuid),
('item_cost::cost_partial_returns_allocate_cumulative_cent_tail','6cea3f8b-4c81-4c82-a88e-6a8dc9a22575'::uuid),
('item_cost::cost_twenty_original_returns_never_exceed_source_quantity','018677f6-f564-40f0-8473-4ff454c538db'::uuid),
('item_cost::cost_twenty_same_commands_return_one_original_result','2540019c-afe3-4d7d-a974-e87ad24f1900'::uuid),
('item_cost::cost_history_orders_effects_after_fences_not_transaction_start','823a3b1e-d6b6-4f97-898b-b66678103c67'::uuid),
('inventory_release::fulfillment_release_once_replay_and_other_source_readback','f5e95d1c-c4a3-465e-bcc1-834ba0b054a9'::uuid),
('inventory_release::fulfillment_release_twenty_concurrent_calls_keep_one_effect','331f7361-2c1b-4e11-aaad-1658d4e6fb71'::uuid)
)
select jsonb_build_object(
 'database',current_database(),'role',current_user,'host',inet_server_addr(),'port',inet_server_port(),
 'probe_schema_exists',exists(select 1 from pg_namespace where nspname='qa_cost_order_migration_probe'),
 'samples',(select jsonb_agg(jsonb_build_object(
  'test_case',s.test_case,'tenant_id',s.tenant_id,
  'cost_entries',(select coalesce(jsonb_agg(jsonb_build_object('kind',c.kind,'qty_micros',c.qty_micros,'amount_cent',c.amount_cent,'estimated',c.estimated,'source_ref',c.source_ref,'created_at',c.created_at,'entry_sequence',c.entry_sequence) order by c.entry_sequence),'[]'::jsonb) from shared_item_costs c where c.tenant_id=s.tenant_id),
  'command_receipts',(select count(*) from command_receipts r where r.tenant_id=s.tenant_id),
  'inventory_balances',(select coalesce(jsonb_agg(jsonb_build_object('on_hand',b.on_hand,'reserved',b.reserved)),'[]'::jsonb) from inventory_balances b where b.tenant_id=s.tenant_id),
  'inventory_ledger',(select jsonb_build_object('on_hand',sum(l.delta_on_hand),'reserved',sum(l.delta_reserved),'release_entries',count(*) filter(where l.reason_type='RESERVATION_RELEASE')) from inventory_ledgers l where l.tenant_id=s.tenant_id),
  'reservation_states',(select jsonb_build_object('reserved',count(*) filter(where r.status='RESERVED'),'released',count(*) filter(where r.status='RELEASED')) from inventory_reservations r where r.tenant_id=s.tenant_id)
 ) order by s.test_case) from selected s)
);
rollback;
