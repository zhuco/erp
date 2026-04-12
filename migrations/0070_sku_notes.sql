-- 0070: SKU notes
alter table skus add column if not exists note text;
