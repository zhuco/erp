create index if not exists idx_operation_logs_frontend_route_path
  on operation_logs(tenant_id, action, (metadata->>'path'), created_at desc, log_id desc)
  where action = 'SYSTEM.FRONTEND_ROUTE_DIAGNOSTIC';

create index if not exists idx_operation_logs_frontend_route_status
  on operation_logs(tenant_id, action, (metadata->>'status'), created_at desc, log_id desc)
  where action = 'SYSTEM.FRONTEND_ROUTE_DIAGNOSTIC';

create index if not exists idx_operation_logs_frontend_route_event_type
  on operation_logs(tenant_id, action, (metadata->>'event_type'), created_at desc, log_id desc)
  where action = 'SYSTEM.FRONTEND_ROUTE_DIAGNOSTIC';
