-- 0042: Print template content format

alter table if exists print_templates
  add column if not exists content_format text not null default 'HTML';
