-- 0167: a disposition cites the inspection, batch, and location.
-- The quality status is written with the quantity ledger. A later row does not rewrite the earlier one.

alter table mfg_quality_dispositions add column if not exists inspection_id uuid;
alter table mfg_quality_dispositions add column if not exists location_id uuid;
alter table mfg_quality_dispositions add column if not exists version_no integer not null default 1;
alter table mfg_quality_dispositions add column if not exists quality_status text not null default '';
