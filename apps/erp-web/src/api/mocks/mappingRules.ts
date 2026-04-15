import dayjs from 'dayjs';

import type {
  MappingRule,
  MappingRuleCreatePayload,
  MappingRuleListQuery,
  MappingRuleListResponse,
  MappingRuleUpdatePayload,
} from '../mappingRules';
import { applyPagination, mockMappingRules, mockTenantId } from './data';

export const getMockMappingRuleList = (
  query: MappingRuleListQuery,
): MappingRuleListResponse => {
  const filtered = mockMappingRules.filter((rule) => {
    if (query.platform && rule.platform !== query.platform) {
      return false;
    }
    if (query.entity_type && rule.entity_type !== query.entity_type) {
      return false;
    }
    if (query.enabled != null && rule.enabled !== query.enabled) {
      return false;
    }
    return true;
  });
  return {
    items: applyPagination(filtered, query.offset, query.limit),
    total: filtered.length,
  };
};

export const createMockMappingRule = (payload: MappingRuleCreatePayload): MappingRule => {
  const now = dayjs().toISOString();
  const record: MappingRule = {
    rule_id: `MAP-RULE-${Date.now()}`,
    tenant_id: mockTenantId,
    platform: payload.platform,
    entity_type: payload.entity_type,
    version: payload.version,
    enabled: payload.enabled ?? true,
    rule_json: payload.rule_json ?? {},
    created_at: now,
    updated_at: now,
  };
  mockMappingRules.unshift(record);
  return record;
};

export const updateMockMappingRule = (
  ruleId: string,
  payload: MappingRuleUpdatePayload,
): MappingRule => {
  const index = mockMappingRules.findIndex((item) => item.rule_id === ruleId);
  const now = dayjs().toISOString();
  const base =
    index >= 0
      ? mockMappingRules[index]
      : {
          rule_id: ruleId,
          tenant_id: mockTenantId,
          platform: 'taobao',
          entity_type: 'SHOP_PRODUCT',
          version: 1,
          enabled: true,
          rule_json: {},
          created_at: now,
          updated_at: now,
        };
  const updated: MappingRule = {
    ...base,
    enabled: payload.enabled ?? base.enabled,
    rule_json: payload.rule_json ?? base.rule_json,
    updated_at: now,
  };
  if (index >= 0) {
    mockMappingRules[index] = updated;
  } else {
    mockMappingRules.unshift(updated);
  }
  return updated;
};

export const getActiveMockApparelRule = (platform?: string) =>
  mockMappingRules.find((rule) => {
    if (!rule.enabled || rule.entity_type !== 'SHOP_PRODUCT') {
      return false;
    }
    if (platform && rule.platform !== platform) {
      return false;
    }
    return rule.rule_json?.industry_profile === 'APPAREL';
  }) ?? null;
