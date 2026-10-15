-- A manual lot choice is stored beside the suggestion. It does not rewrite the rule.

create table if not exists shared_allocation_choices (
  tenant_id uuid not null,
  choice_id uuid not null,
  suggestion_id uuid not null,
  chosen_batch text not null,
  reason text not null,
  decision text not null,
  created_at timestamptz not null default now(),
  primary key (tenant_id, choice_id)
);
