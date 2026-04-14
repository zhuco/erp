import { apiBaseUrl, useMocks } from '@/utils/env';
import { buildAuthHeaders } from '@/api/client';
import { ApiError, createApiError, handleApiError, isErrorResponse } from '@/api/errors';

export type PermissionDefinition = {
  code: string;
  name: string;
  module: string;
  description: string;
};

export type Role = {
  role_id: string;
  tenant_id: string;
  name: string;
  created_at: string;
};

export type RoleCreateRequest = {
  name: string;
  permissions?: string[];
};

export type RoleUpdateRequest = {
  name: string;
};

export type RolePermissionsRequest = {
  permissions: string[];
};

export type User = {
  user_id: string;
  tenant_id: string;
  email: string;
  status: string;
  created_at: string;
};

export type UserCreateRequest = {
  email: string;
  password: string;
  status?: string;
  role_ids?: string[];
};

export type UserUpdateRequest = {
  email?: string;
  password?: string;
  status?: string;
};

export type UserRolesRequest = {
  role_ids: string[];
};

export type UserRolesResponse = {
  roles: Role[];
};

export type DataScopeResponse = {
  shop_ids: string[];
  warehouse_ids: string[];
};

export type DataScopeUpdateRequest = {
  shop_ids: string[];
  warehouse_ids: string[];
};

export type AuditLog = {
  audit_id: string;
  tenant_id: string;
  user_id?: string | null;
  action: string;
  entity_type: string;
  entity_id?: string | null;
  before_json?: Record<string, unknown> | null;
  after_json?: Record<string, unknown> | null;
  ip?: string | null;
  ua?: string | null;
  created_at: string;
};

export type OperationLog = {
  log_id: string;
  tenant_id: string;
  actor_type: string;
  actor_user_id?: string | null;
  action: string;
  ref_type?: string | null;
  ref_id?: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
};

export type CursorListResponse<T> = {
  data: T[];
  next_cursor?: string | null;
};

export type AuditLogListQuery = {
  action?: string;
  entity_type?: string;
  entity_id?: string;
  user_id?: string;
  created_from?: string;
  created_to?: string;
  cursor?: string;
  limit?: number;
};

export type OperationLogListQuery = {
  action?: string;
  ref_type?: string;
  ref_id?: string;
  actor_user_id?: string;
  created_from?: string;
  created_to?: string;
  cursor?: string;
  limit?: number;
};

export type Setting = {
  setting_id: string;
  tenant_id: string;
  scope_type: string;
  scope_id?: string | null;
  key: string;
  value_json: Record<string, unknown>;
  version: number;
  updated_at: string;
};

export type SettingListQuery = {
  scope_type?: string;
  scope_id?: string;
  key?: string;
};

export type SettingUpsertRequest = {
  scope_type: string;
  scope_id?: string | null;
  key: string;
  value: Record<string, unknown>;
  expected_version?: number | null;
};

export type MetricsSnapshot = Record<string, number>;

export type FrontendRouteDiagnosticEventType =
  | 'loading-ready'
  | 'loading-timeout'
  | 'loading-error'
  | 'prefetch-ready'
  | 'prefetch-error'
  | 'prefetch-skipped';

export type FrontendRouteDiagnosticStatus = 'ready' | 'timeout' | 'error' | 'loading' | 'idle';

export type FrontendRouteDiagnosticReport = {
  session_id: string;
  path: string;
  route_label?: string;
  event_type: FrontendRouteDiagnosticEventType;
  status: FrontendRouteDiagnosticStatus;
  duration_ms?: number;
  prefetch_status?: 'prefetching' | 'ready' | 'error' | 'skipped';
  prefetch_source?: 'hover' | 'open' | 'search' | 'recommendation' | 'programmatic';
  prefetch_duration_ms?: number;
  error_summary?: string;
  detail?: string;
  event_at: number;
  created_at?: string;
  page_url: string;
  build_version: string;
  user_agent: string;
};

export type FrontendRouteDiagnosticsQuery = {
  pathKeyword?: string;
  status?: FrontendRouteDiagnosticStatus | 'all';
  eventType?: FrontendRouteDiagnosticEventType | 'all';
  windowHours?: number;
  limit?: number;
};

export type FrontendRouteDiagnosticsSummary = {
  totalEvents: number;
  uniqueRoutes: number;
  readyEvents: number;
  timeoutEvents: number;
  errorEvents: number;
  slowEvents: number;
  prefetchHits: number;
  affectedSessions: number;
  measuredFirstLoads: number;
  successfulFirstLoads: number;
  firstLoadAvgDurationMs?: number;
  prefetchShortCircuitCount: number;
};

export type FrontendRouteDiagnosticsPathSummary = {
  path: string;
  routeLabel?: string;
  totalEvents: number;
  readyEvents: number;
  timeoutEvents: number;
  errorEvents: number;
  slowEvents: number;
  prefetchHits: number;
  avgDurationMs?: number;
  maxDurationMs?: number;
  measuredFirstLoads: number;
  successfulFirstLoads: number;
  firstLoadAvgDurationMs?: number;
  prefetchShortCircuitCount: number;
  lastSeenAt: string;
};

export type FrontendRouteDiagnosticsResponse = {
  summary: FrontendRouteDiagnosticsSummary;
  paths: FrontendRouteDiagnosticsPathSummary[];
  events: FrontendRouteDiagnosticReport[];
};

export type FrontendRouteDiagnosticsReportResponse = {
  accepted: number;
};

const REQUEST_TIMEOUT_MS = 15000;

const requestJson = async <T>(input: RequestInfo, init: RequestInit, message: string) => {
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const response = await fetch(input, { ...init, signal: controller.signal });
    let payload: unknown = undefined;
    try {
      payload = await response.json();
    } catch {
      payload = undefined;
    }
    if (!response.ok) {
      handleApiError(response.status);
      const errorMessage = isErrorResponse(payload)
        ? payload.message
        : response.statusText || message;
      throw createApiError(errorMessage, response.status, payload);
    }
    return payload as T;
  } catch (error) {
    if (error instanceof ApiError) {
      throw error;
    }
    if (error instanceof DOMException && error.name === 'AbortError') {
      handleApiError(408);
      throw createApiError('请求超时，请稍后重试', 408);
    }
    throw createApiError(message, undefined);
  } finally {
    window.clearTimeout(timeout);
  }
};

const requestText = async (input: RequestInfo, init: RequestInit, message: string) => {
  const controller = new AbortController();
  const timeout = window.setTimeout(() => controller.abort(), REQUEST_TIMEOUT_MS);
  try {
    const response = await fetch(input, { ...init, signal: controller.signal });
    const payload = await response.text();
    if (!response.ok) {
      handleApiError(response.status);
      throw createApiError(response.statusText || message, response.status, payload);
    }
    return payload;
  } catch (error) {
    if (error instanceof ApiError) {
      throw error;
    }
    if (error instanceof DOMException && error.name === 'AbortError') {
      handleApiError(408);
      throw createApiError('请求超时，请稍后重试', 408);
    }
    throw createApiError(message, undefined);
  } finally {
    window.clearTimeout(timeout);
  }
};

const buildQueryString = (query: Record<string, string | undefined>) => {
  const params = new URLSearchParams();
  Object.entries(query).forEach(([key, value]) => {
    if (!value) {
      return;
    }
    params.set(key, value);
  });
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

const buildFrontendRouteDiagnosticsQueryString = (query: FrontendRouteDiagnosticsQuery) => {
  const params = new URLSearchParams();
  if (query.pathKeyword?.trim()) {
    params.set('path_keyword', query.pathKeyword.trim());
  }
  if (query.status && query.status !== 'all') {
    params.set('status', query.status);
  }
  if (query.eventType && query.eventType !== 'all') {
    params.set('event_type', query.eventType);
  }
  if (typeof query.windowHours === 'number' && Number.isFinite(query.windowHours)) {
    params.set('window_hours', String(Math.max(1, Math.round(query.windowHours))));
  }
  if (typeof query.limit === 'number' && Number.isFinite(query.limit)) {
    params.set('limit', String(Math.max(1, Math.round(query.limit))));
  }
  const serialized = params.toString();
  return serialized ? `?${serialized}` : '';
};

const parseMetricsText = (text: string): MetricsSnapshot => {
  const metrics: MetricsSnapshot = {};
  text.split('\n').forEach((line) => {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) {
      return;
    }
    const [name, value] = trimmed.split(/\s+/);
    if (!name || !value) {
      return;
    }
    const parsed = Number(value);
    if (!Number.isNaN(parsed)) {
      metrics[name] = parsed;
    }
  });
  return metrics;
};

export const listPermissions = async (tenantId: string): Promise<PermissionDefinition[]> => {
  void tenantId;
  if (useMocks) {
    return [];
  }
  return requestJson<PermissionDefinition[]>(
    `${apiBaseUrl}/admin/permissions`,
    { headers: buildAuthHeaders() },
    '获取权限目录失败'
  );
};

export const listRoles = async (tenantId: string): Promise<Role[]> => {
  void tenantId;
  if (useMocks) {
    return [];
  }
  return requestJson<Role[]>(
    `${apiBaseUrl}/admin/roles`,
    { headers: buildAuthHeaders() },
    '获取角色列表失败'
  );
};

export const createRole = async (
  tenantId: string,
  payload: RoleCreateRequest
): Promise<Role> => {
  if (useMocks) {
    return {
      role_id: `${Date.now()}`,
      tenant_id: tenantId,
      name: payload.name,
      created_at: new Date().toISOString()
    };
  }
  return requestJson<Role>(
    `${apiBaseUrl}/admin/roles`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建角色失败'
  );
};

export const updateRole = async (
  tenantId: string,
  roleId: string,
  payload: RoleUpdateRequest
): Promise<Role> => {
  if (useMocks) {
    return {
      role_id: roleId,
      tenant_id: tenantId,
      name: payload.name,
      created_at: new Date().toISOString()
    };
  }
  return requestJson<Role>(
    `${apiBaseUrl}/admin/roles/${roleId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新角色失败'
  );
};

export const listRolePermissions = async (
  tenantId: string,
  roleId: string
): Promise<string[]> => {
  if (useMocks) {
    return [];
  }
  return requestJson<string[]>(
    `${apiBaseUrl}/admin/roles/${roleId}/permissions`,
    { headers: buildAuthHeaders() },
    '获取角色权限失败'
  );
};

export const updateRolePermissions = async (
  tenantId: string,
  roleId: string,
  payload: RolePermissionsRequest
): Promise<string[]> => {
  if (useMocks) {
    return payload.permissions;
  }
  return requestJson<string[]>(
    `${apiBaseUrl}/admin/roles/${roleId}/permissions`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新角色权限失败'
  );
};

export const listUsers = async (
  tenantId: string,
  query: { email?: string; status?: string }
): Promise<User[]> => {
  if (useMocks) {
    return [];
  }
  return requestJson<User[]>(
    `${apiBaseUrl}/admin/users${buildQueryString({
      email: query.email,
      status: query.status
    })}`,
    { headers: buildAuthHeaders() },
    '获取用户列表失败'
  );
};

export const createUser = async (
  tenantId: string,
  payload: UserCreateRequest
): Promise<User> => {
  if (useMocks) {
    return {
      user_id: `${Date.now()}`,
      tenant_id: tenantId,
      email: payload.email,
      status: payload.status ?? 'ACTIVE',
      created_at: new Date().toISOString()
    };
  }
  return requestJson<User>(
    `${apiBaseUrl}/admin/users`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '创建用户失败'
  );
};

export const updateUser = async (
  tenantId: string,
  userId: string,
  payload: UserUpdateRequest
): Promise<User> => {
  if (useMocks) {
    return {
      user_id: userId,
      tenant_id: tenantId,
      email: payload.email ?? 'mock@erp',
      status: payload.status ?? 'ACTIVE',
      created_at: new Date().toISOString()
    };
  }
  return requestJson<User>(
    `${apiBaseUrl}/admin/users/${userId}`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新用户失败'
  );
};

export const getUserRoles = async (
  tenantId: string,
  userId: string
): Promise<UserRolesResponse> => {
  if (useMocks) {
    return { roles: [] };
  }
  return requestJson<UserRolesResponse>(
    `${apiBaseUrl}/admin/users/${userId}/roles`,
    { headers: buildAuthHeaders() },
    '获取用户角色失败'
  );
};

export const updateUserRoles = async (
  tenantId: string,
  userId: string,
  payload: UserRolesRequest
): Promise<string[]> => {
  if (useMocks) {
    return payload.role_ids;
  }
  return requestJson<string[]>(
    `${apiBaseUrl}/admin/users/${userId}/roles`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新用户角色失败'
  );
};

export const getUserDataScopes = async (
  tenantId: string,
  userId: string
): Promise<DataScopeResponse> => {
  if (useMocks) {
    return { shop_ids: [], warehouse_ids: [] };
  }
  return requestJson<DataScopeResponse>(
    `${apiBaseUrl}/admin/users/${userId}/data-scopes`,
    { headers: buildAuthHeaders() },
    '获取数据权限失败'
  );
};

export const updateUserDataScopes = async (
  tenantId: string,
  userId: string,
  payload: DataScopeUpdateRequest
): Promise<string> => {
  if (useMocks) {
    return 'ok';
  }
  return requestJson<string>(
    `${apiBaseUrl}/admin/users/${userId}/data-scopes`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新数据权限失败'
  );
};

export const listAuditLogs = async (
  tenantId: string,
  query: AuditLogListQuery
): Promise<CursorListResponse<AuditLog>> => {
  if (useMocks) {
    return { data: [], next_cursor: null };
  }
  return requestJson<CursorListResponse<AuditLog>>(
    `${apiBaseUrl}/admin/audit-logs${buildQueryString({
      action: query.action,
      entity_type: query.entity_type,
      entity_id: query.entity_id,
      user_id: query.user_id,
      created_from: query.created_from,
      created_to: query.created_to,
      cursor: query.cursor,
      limit: query.limit ? String(query.limit) : undefined
    })}`,
    { headers: buildAuthHeaders() },
    '获取审计日志失败'
  );
};

export const listOperationLogs = async (
  tenantId: string,
  query: OperationLogListQuery
): Promise<CursorListResponse<OperationLog>> => {
  if (useMocks) {
    return { data: [], next_cursor: null };
  }
  return requestJson<CursorListResponse<OperationLog>>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/operation-logs${buildQueryString({
      action: query.action,
      ref_type: query.ref_type,
      ref_id: query.ref_id,
      actor_user_id: query.actor_user_id,
      created_from: query.created_from,
      created_to: query.created_to,
      cursor: query.cursor,
      limit: query.limit ? String(query.limit) : undefined
    })}`,
    { headers: buildAuthHeaders() },
    '获取操作日志失败'
  );
};

export const listSettings = async (
  tenantId: string,
  query: SettingListQuery
): Promise<Setting[]> => {
  if (useMocks) {
    return [];
  }
  return requestJson<Setting[]>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/settings${buildQueryString({
      scope_type: query.scope_type,
      scope_id: query.scope_id,
      key: query.key
    })}`,
    { headers: buildAuthHeaders() },
    '获取配置失败'
  );
};

export const upsertSetting = async (
  tenantId: string,
  payload: SettingUpsertRequest
): Promise<Setting> => {
  if (useMocks) {
    return {
      setting_id: `${Date.now()}`,
      tenant_id: tenantId,
      scope_type: payload.scope_type,
      scope_id: payload.scope_id,
      key: payload.key,
      value_json: payload.value,
      version: payload.expected_version ? payload.expected_version + 1 : 1,
      updated_at: new Date().toISOString()
    };
  }
  return requestJson<Setting>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/settings`,
    {
      method: 'PUT',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload)
    },
    '更新配置失败'
  );
};

export const fetchMetrics = async (): Promise<MetricsSnapshot> => {
  if (useMocks) {
    return {};
  }
  const text = await requestText(
    `${apiBaseUrl}/metrics`,
    { headers: buildAuthHeaders() },
    '获取监控指标失败'
  );
  return parseMetricsText(text);
};

export const reportFrontendRouteDiagnostics = async (
  tenantId: string,
  payload: { events: FrontendRouteDiagnosticReport[] }
): Promise<FrontendRouteDiagnosticsReportResponse> => {
  if (payload.events.length === 0) {
    return { accepted: 0 };
  }
  if (useMocks) {
    return { accepted: payload.events.length };
  }
  return requestJson<FrontendRouteDiagnosticsReportResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/frontend-route-diagnostics`,
    {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...buildAuthHeaders()
      },
      body: JSON.stringify(payload),
      keepalive: true
    },
    '上报前端路由观测失败'
  );
};

export const getFrontendRouteDiagnostics = async (
  tenantId: string,
  query: FrontendRouteDiagnosticsQuery
): Promise<FrontendRouteDiagnosticsResponse> => {
  if (useMocks) {
    return {
      summary: {
        totalEvents: 0,
        uniqueRoutes: 0,
        readyEvents: 0,
        timeoutEvents: 0,
        errorEvents: 0,
        slowEvents: 0,
        prefetchHits: 0,
        affectedSessions: 0,
        measuredFirstLoads: 0,
        successfulFirstLoads: 0,
        prefetchShortCircuitCount: 0
      },
      paths: [],
      events: []
    };
  }
  return requestJson<FrontendRouteDiagnosticsResponse>(
    `${apiBaseUrl}/v1/tenants/${tenantId}/system/frontend-route-diagnostics${buildFrontendRouteDiagnosticsQueryString(query)}`,
    { headers: buildAuthHeaders() },
    '获取前端路由观测失败'
  );
};
