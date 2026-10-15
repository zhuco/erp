-- Preserve the previous deterministic order for historical entries, then assign
-- new entries at INSERT time after their item fence (not transaction start time).
lock table shared_item_costs in access exclusive mode;

alter table shared_item_costs add column if not exists entry_sequence bigint;
create sequence if not exists shared_item_cost_entry_sequence;

with ranked as (
  select tenant_id, entry_id,
    row_number() over (order by created_at, entry_id, tenant_id)
      + coalesce((select max(entry_sequence) from shared_item_costs), 0) as sequence_value
  from shared_item_costs
  where entry_sequence is null
)
update shared_item_costs c set entry_sequence = ranked.sequence_value
from ranked where c.tenant_id = ranked.tenant_id and c.entry_id = ranked.entry_id;

select setval('shared_item_cost_entry_sequence',
  coalesce(max(entry_sequence), 1), max(entry_sequence) is not null)
from shared_item_costs;

alter sequence shared_item_cost_entry_sequence owned by shared_item_costs.entry_sequence;
alter table shared_item_costs alter column entry_sequence
  set default nextval('shared_item_cost_entry_sequence');
alter table shared_item_costs alter column entry_sequence set not null;

create unique index if not exists shared_item_cost_entry_sequence_unique
  on shared_item_costs(entry_sequence);
create index if not exists shared_item_cost_history_order
  on shared_item_costs(tenant_id, item_code, entry_sequence);
create index if not exists shared_item_cost_original_source
  on shared_item_costs(tenant_id, item_code, source_ref, kind);
