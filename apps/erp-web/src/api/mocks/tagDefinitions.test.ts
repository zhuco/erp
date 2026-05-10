import { describe, expect, it } from 'vitest';

import {
  createMockOrderTagDefinition,
  createMockSkuTagDefinition,
  deleteMockOrderTagDefinition,
  deleteMockSkuTagDefinition,
  listMockOrderTagDefinitions,
  listMockSkuTagDefinitions,
  updateMockOrderTagDefinition,
  updateMockSkuTagDefinition,
} from './tagDefinitions';

describe('tagDefinitions mocks', () => {
  it('provides stable seeded order tags with filtering and pagination', () => {
    const seeded = listMockOrderTagDefinitions({
      offset: 0,
      limit: 50,
    });
    const filtered = listMockOrderTagDefinitions({
      keyword: 'vip',
      offset: 0,
      limit: 50,
    });
    const paged = listMockOrderTagDefinitions({
      offset: 0,
      limit: 3,
    });

    expect(seeded.total).toBeGreaterThanOrEqual(6);
    expect(seeded.items.some((item) => item.auto_apply)).toBe(true);
    expect(filtered.total).toBeGreaterThanOrEqual(1);
    expect(filtered.items.every((item) => item.name.toLowerCase().includes('vip'))).toBe(true);
    expect(paged.items).toHaveLength(3);
  });

  it('supports order tag create, update, and delete in mock mode', () => {
    const created = createMockOrderTagDefinition('tenant-test', {
      name: 'qa-watch',
      color: '#123456',
      priority: 90,
      status: 'ACTIVE',
      auto_apply: true,
      conditions_json: [{ field_path: 'risk_level', op: '=', value: 'QA' }],
    });
    const updated = updateMockOrderTagDefinition('tenant-test', created.tag_id, {
      name: 'qa-watch-updated',
      status: 'DISABLED',
      auto_apply: false,
    });
    const deleted = deleteMockOrderTagDefinition('tenant-test', created.tag_id);
    const remaining = listMockOrderTagDefinitions({
      keyword: 'qa-watch-updated',
      offset: 0,
      limit: 20,
    });

    expect(updated.name).toBe('qa-watch-updated');
    expect(updated.status).toBe('DISABLED');
    expect(updated.auto_apply).toBe(false);
    expect(deleted.tag_id).toBe(created.tag_id);
    expect(remaining.total).toBe(0);
  });

  it('provides stable seeded sku tags with filtering and pagination', () => {
    const seeded = listMockSkuTagDefinitions({
      offset: 0,
      limit: 50,
    });
    const filtered = listMockSkuTagDefinitions({
      keyword: 'fragile',
      offset: 0,
      limit: 50,
    });
    const paged = listMockSkuTagDefinitions({
      offset: 2,
      limit: 2,
    });

    expect(seeded.total).toBeGreaterThanOrEqual(6);
    expect(filtered.total).toBeGreaterThanOrEqual(1);
    expect(filtered.items.every((item) => item.name.toLowerCase().includes('fragile'))).toBe(true);
    expect(paged.items).toHaveLength(2);
  });

  it('supports sku tag create, update, and delete in mock mode', () => {
    const created = createMockSkuTagDefinition('tenant-test', {
      name: 'qa-fragile',
      color: '#654321',
      priority: 33,
      status: 'ACTIVE',
    });
    const updated = updateMockSkuTagDefinition('tenant-test', created.tag_id, {
      name: 'qa-fragile-updated',
      status: 'DISABLED',
    });
    const deleted = deleteMockSkuTagDefinition('tenant-test', created.tag_id);
    const remaining = listMockSkuTagDefinitions({
      keyword: 'qa-fragile-updated',
      offset: 0,
      limit: 20,
    });

    expect(updated.name).toBe('qa-fragile-updated');
    expect(updated.status).toBe('DISABLED');
    expect(deleted.tag_id).toBe(created.tag_id);
    expect(remaining.total).toBe(0);
  });
});
