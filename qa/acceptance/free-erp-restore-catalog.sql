SELECT json_build_object(
 'roots',(SELECT coalesce(json_agg(c.relname ORDER BY c.relname),'[]'::json)
 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
 WHERE n.nspname='public' AND c.relkind IN ('r','p') AND NOT c.relispartition),
 'sequences',(SELECT coalesce(json_agg(c.relname ORDER BY c.relname),'[]'::json)
 FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
 WHERE n.nspname='public' AND c.relkind='S'),
 'relations',(SELECT coalesce(json_agg(json_build_object(
 'name',c.relname,'kind',c.relkind,'owner',pg_get_userbyid(c.relowner),
 'acl',(SELECT array_agg(v::text ORDER BY v::text) FROM unnest(c.relacl) v),
 'rls',c.relrowsecurity,'force_rls',c.relforcerowsecurity,
 'partition',pg_get_expr(c.relpartbound,c.oid),
 'parents',(SELECT array_agg(p.relname ORDER BY p.relname) FROM pg_inherits i JOIN pg_class p ON p.oid=i.inhparent WHERE i.inhrelid=c.oid),
 'view',CASE WHEN c.relkind IN ('v','m') THEN pg_get_viewdef(c.oid,false) END,
 'columns',(SELECT json_agg(json_build_object(
 'name',a.attname,'type',format_type(a.atttypid,a.atttypmod),'notnull',a.attnotnull,
 'identity',a.attidentity,'generated',a.attgenerated,
 'collation',CASE WHEN a.attcollation<>0 THEN a.attcollation::regcollation::text END,
 'default',pg_get_expr(d.adbin,d.adrelid)) ORDER BY a.attnum)
 FROM pg_attribute a LEFT JOIN pg_attrdef d ON d.adrelid=a.attrelid AND d.adnum=a.attnum
 WHERE a.attrelid=c.oid AND a.attnum>0 AND NOT a.attisdropped)
 ) ORDER BY c.relname),'[]'::json) FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace
 WHERE n.nspname='public' AND c.relkind IN ('r','p','v','m','S','f')),
 'constraints',(SELECT coalesce(json_agg(json_build_object(
 'name',c.conname,'relation',c.conrelid::regclass::text,'type',c.contype,
 'definition',pg_get_constraintdef(c.oid,false),'validated',c.convalidated)
 ORDER BY c.conrelid::regclass::text,c.conname),'[]'::json)
 FROM pg_constraint c JOIN pg_namespace n ON n.oid=c.connamespace WHERE n.nspname='public'),
 'indexes',(SELECT coalesce(json_agg(json_build_object(
 'relation',i.indrelid::regclass::text,'name',c.relname,'definition',pg_get_indexdef(c.oid),
 'valid',i.indisvalid,'ready',i.indisready) ORDER BY i.indrelid::regclass::text,c.relname),'[]'::json)
 FROM pg_index i JOIN pg_class c ON c.oid=i.indexrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public'),
 'triggers',(SELECT coalesce(json_agg(json_build_object(
 'relation',t.tgrelid::regclass::text,'name',t.tgname,'enabled',t.tgenabled,
 'definition',pg_get_triggerdef(t.oid,false)) ORDER BY t.tgrelid::regclass::text,t.tgname),'[]'::json)
 FROM pg_trigger t JOIN pg_class c ON c.oid=t.tgrelid JOIN pg_namespace n ON n.oid=c.relnamespace
 WHERE n.nspname='public' AND NOT t.tgisinternal),
 'functions',(SELECT coalesce(json_agg(json_build_object(
 'name',p.proname,'identity',pg_get_function_identity_arguments(p.oid),
 'owner',pg_get_userbyid(p.proowner),'acl',(SELECT array_agg(v::text ORDER BY v::text) FROM unnest(p.proacl) v),
 'definition',pg_get_functiondef(p.oid)) ORDER BY p.proname,pg_get_function_identity_arguments(p.oid)),'[]'::json)
 FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace WHERE n.nspname='public' AND p.prokind IN ('f','p')),
 'types',(SELECT coalesce(json_agg(json_build_object(
 'name',t.typname,'kind',t.typtype,'base',format_type(t.typbasetype,t.typtypmod),
 'default',t.typdefault,'notnull',t.typnotnull,'owner',pg_get_userbyid(t.typowner),
 'enum',(SELECT json_agg(e.enumlabel ORDER BY e.enumsortorder) FROM pg_enum e WHERE e.enumtypid=t.oid)
 ) ORDER BY t.typname),'[]'::json) FROM pg_type t JOIN pg_namespace n ON n.oid=t.typnamespace
 WHERE n.nspname='public' AND t.typtype IN ('e','d')),
 'policies',(SELECT coalesce(json_agg(json_build_object(
 'relation',p.polrelid::regclass::text,'name',p.polname,'command',p.polcmd,
 'permissive',p.polpermissive,'roles',(SELECT array_agg(CASE WHEN r=0 THEN 'public' ELSE pg_get_userbyid(r) END ORDER BY r) FROM unnest(p.polroles) r),
 'using',pg_get_expr(p.polqual,p.polrelid),'check',pg_get_expr(p.polwithcheck,p.polrelid)) ORDER BY p.polrelid::regclass::text,p.polname),'[]'::json)
 FROM pg_policy p JOIN pg_class c ON c.oid=p.polrelid JOIN pg_namespace n ON n.oid=c.relnamespace WHERE n.nspname='public'),
 'extensions',(SELECT json_agg(json_build_object('name',e.extname,'version',e.extversion,'schema',n.nspname,'owner',pg_get_userbyid(e.extowner)) ORDER BY e.extname)
 FROM pg_extension e JOIN pg_namespace n ON n.oid=e.extnamespace),
 'sequence_parameters',(SELECT coalesce(json_agg(to_jsonb(s)-'last_value' ORDER BY sequencename),'[]'::json) FROM pg_sequences s WHERE schemaname='public')
);
