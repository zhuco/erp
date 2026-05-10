import { fetchWithAuth } from '@/api/client';
import { createApiError, handleApiError } from '@/api/errors';
import { useSessionStore } from '@/features/sessionStore';
import { apiBaseUrl, useMocks } from '@/utils/env';

export type FeatureFlagKey =
  | 'orders.list'
  | 'orders.ship'
  | 'orders.print_waybill'
  | 'orders.refund_sync'
  | 'orders.template_fetch'
  | 'aftersales.list'
  | 'flows.order_auto_download'
  | 'flows.merge_outside_ship_time'
  | 'flows.inventory_sync'
  | 'flows.aftersales_download'
  | 'flows.logistics_upload'
  | 'flows.waybill_reserve'
  | 'flows.auto_freeze'
  | 'flows.pack_performance'
  | 'flows.pick_recommend'
  | 'flows.stock_count_suggest'
  | 'directory.logistics_price'
  | 'directory.tag_system'
  | 'inventory.enabled'
  | 'inventory_insights'
  | 'wms.enable_wave'
  | 'wms.enable_reserve_tracking'
  | 'wms.enable_outbound_inspection'
  | 'wms.enable_pack_performance'
  | 'wms.enable_shipment_cancel'
  | 'print.enable_local_agent'
  | 'cost.enabled'
  | 'system.read_only'
  | 'ui.enable_stats';

export type FeatureFlagState = 'enabled' | 'disabled' | 'unavailable';

export type FeatureFlags = Record<FeatureFlagKey, FeatureFlagState>;

const PACK_PERFORMANCE_LEGACY_KEY: FeatureFlagKey = 'flows.pack_performance';
const PACK_PERFORMANCE_CANONICAL_KEY: FeatureFlagKey = 'wms.enable_pack_performance';

export const defaultFeatureFlags: FeatureFlags = {
  'orders.list': 'enabled',
  'orders.ship': 'enabled',
  'orders.print_waybill': 'enabled',
  'orders.refund_sync': 'enabled',
  'orders.template_fetch': 'enabled',
  'aftersales.list': 'enabled',
  'flows.order_auto_download': 'enabled',
  'flows.merge_outside_ship_time': 'enabled',
  'flows.inventory_sync': 'enabled',
  'flows.aftersales_download': 'enabled',
  'flows.logistics_upload': 'enabled',
  'flows.waybill_reserve': 'enabled',
  'flows.auto_freeze': 'enabled',
  'flows.pack_performance': 'enabled',
  'flows.pick_recommend': 'enabled',
  'flows.stock_count_suggest': 'enabled',
  'directory.logistics_price': 'enabled',
  'directory.tag_system': 'enabled',
  'inventory.enabled': 'enabled',
  'inventory_insights': 'enabled',
  'wms.enable_wave': 'enabled',
  'wms.enable_reserve_tracking': 'enabled',
  'wms.enable_outbound_inspection': 'enabled',
  'wms.enable_pack_performance': 'enabled',
  'wms.enable_shipment_cancel': 'enabled',
  'print.enable_local_agent': 'disabled',
  'cost.enabled': 'enabled',
  'system.read_only': 'disabled',
  'ui.enable_stats': 'enabled'
};

const normalizeFeatureFlags = (flags?: Partial<FeatureFlags>): FeatureFlags => ({
  ...defaultFeatureFlags,
  ...(flags ?? {}),
  // 统一收口：pack 性能开关以 wms.enable_pack_performance 为准，
  // 并同步到旧键 flows.pack_performance，避免页面切换期行为分叉。
  [PACK_PERFORMANCE_CANONICAL_KEY]:
    flags?.[PACK_PERFORMANCE_CANONICAL_KEY] ??
    flags?.[PACK_PERFORMANCE_LEGACY_KEY] ??
    defaultFeatureFlags[PACK_PERFORMANCE_CANONICAL_KEY],
  [PACK_PERFORMANCE_LEGACY_KEY]:
    flags?.[PACK_PERFORMANCE_CANONICAL_KEY] ??
    flags?.[PACK_PERFORMANCE_LEGACY_KEY] ??
    defaultFeatureFlags[PACK_PERFORMANCE_CANONICAL_KEY]
});

const resolveCanonicalFeatureKey = (feature: FeatureFlagKey): FeatureFlagKey =>
  feature === PACK_PERFORMANCE_LEGACY_KEY ? PACK_PERFORMANCE_CANONICAL_KEY : feature;

export const fetchFeatureFlags = async (tenantId: string): Promise<FeatureFlags> => {
  if (useMocks) {
    return defaultFeatureFlags;
  }
  try {
    // TODO: replace with backend /feature_flags API once available.
    const response = await fetchWithAuth(
      `${apiBaseUrl}/v1/tenants/${tenantId}/feature-flags`
    );
    if (!response.ok) {
      throw new Error('fetch feature flags failed');
    }
    const data = (await response.json()) as Partial<FeatureFlags>;
    return normalizeFeatureFlags(data);
  } catch {
    return defaultFeatureFlags;
  }
};

export type InventoryFlagResponse = {
  inventory_enabled: boolean;
};

export type FeatureFlagsUpdateResponse = {
  flags: Partial<FeatureFlags>;
};

export const updateInventoryEnabled = async (
  tenantId: string,
  inventoryEnabled: boolean
): Promise<InventoryFlagResponse> => {
  const response = await fetchWithAuth(
    `${apiBaseUrl}/v1/tenants/${tenantId}/feature-flags/inventory`,
    {
      method: 'PATCH',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({ inventory_enabled: inventoryEnabled })
    }
  );
  let payload: unknown = undefined;
  try {
    payload = await response.json();
  } catch {
    payload = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('更新库存开关失败', response.status, payload);
  }
  return payload as InventoryFlagResponse;
};

export const updateFeatureFlags = async (
  tenantId: string,
  flags: Record<string, boolean>
): Promise<FeatureFlagsUpdateResponse> => {
  const response = await fetchWithAuth(`${apiBaseUrl}/v1/tenants/${tenantId}/feature-flags`, {
    method: 'PUT',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify({ flags })
  });
  let payload: unknown = undefined;
  try {
    payload = await response.json();
  } catch {
    payload = undefined;
  }
  if (!response.ok) {
    handleApiError(response.status);
    throw createApiError('更新流程开关失败', response.status, payload);
  }
  return payload as FeatureFlagsUpdateResponse;
};

export type FeatureFlagDecision = {
  enabled: boolean;
  reason?: string;
};

export const getFeatureFlagDecision = (
  flags: FeatureFlags,
  feature: FeatureFlagKey
): FeatureFlagDecision => {
  const normalizedFlags = normalizeFeatureFlags(flags);
  const canonicalFeature = resolveCanonicalFeatureKey(feature);
  const state = normalizedFlags[canonicalFeature] ?? 'unavailable';
  if (state === 'enabled') {
    return { enabled: true };
  }
  if (state === 'disabled') {
    return { enabled: false, reason: '灰度关闭' };
  }
  return { enabled: false, reason: '未开通' };
};

export const isFeatureEnabled = (flags: FeatureFlags, feature: FeatureFlagKey): boolean =>
  getFeatureFlagDecision(flags, feature).enabled;

export const useFeatureFlag = (feature: FeatureFlagKey): FeatureFlagDecision => {
  const featureFlags = useSessionStore((state) => state.featureFlags);
  return getFeatureFlagDecision(featureFlags, feature);
};
