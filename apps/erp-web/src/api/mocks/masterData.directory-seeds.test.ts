import { describe, expect, it } from 'vitest';

import {
  createMockWarehouse,
  deleteMockWarehouse,
  getMockCustomerCategoryList,
  getMockCustomerTagList,
  getMockMemberLevelList,
  getMockSupplierList,
  getMockWarehouseList,
  updateMockWarehouse,
} from './masterData';

describe('masterData directory seeds', () => {
  it('provides stable warehouse seeds with status coverage and keyword filtering', () => {
    const seeded = getMockWarehouseList({
      offset: 0,
      limit: 50,
    });
    const filtered = getMockWarehouseList({
      keyword: 'east',
      offset: 0,
      limit: 50,
    });

    expect(seeded.total).toBeGreaterThanOrEqual(6);
    expect(seeded.items.some((item) => item.status === 'ACTIVE')).toBe(true);
    expect(seeded.items.some((item) => item.status === 'DISABLED')).toBe(true);
    expect(filtered.total).toBeGreaterThanOrEqual(1);
    expect(
      filtered.items.every(
        (item) =>
          item.name.toLowerCase().includes('east') || item.code.toLowerCase().includes('east'),
      ),
    ).toBe(true);
  });

  it('updates and deletes warehouses against the same mock dataset', () => {
    const created = createMockWarehouse({
      name: 'qa-hub',
      code: 'QA-HUB',
      status: 'ACTIVE',
      is_default: false,
    });
    const updated = updateMockWarehouse(created.warehouse_id, {
      name: 'qa-hub-updated',
      code: 'QA-HUB-2',
      status: 'DISABLED',
      is_default: false,
    });
    const deleted = deleteMockWarehouse(created.warehouse_id);
    const remaining = getMockWarehouseList({
      keyword: 'qa-hub-updated',
      offset: 0,
      limit: 20,
    });

    expect(updated.name).toBe('qa-hub-updated');
    expect(updated.code).toBe('QA-HUB-2');
    expect(updated.status).toBe('DISABLED');
    expect(deleted.warehouse_id).toBe(created.warehouse_id);
    expect(remaining.total).toBe(0);
  });

  it('provides stable supplier seeds that can be found by contact and settlement terms', () => {
    const seeded = getMockSupplierList({
      offset: 0,
      limit: 50,
    });
    const contactFiltered = getMockSupplierList({
      keyword: 'linda',
      offset: 0,
      limit: 50,
    });
    const settlementFiltered = getMockSupplierList({
      keyword: 'net 45',
      offset: 0,
      limit: 50,
    });

    expect(seeded.total).toBeGreaterThanOrEqual(6);
    expect(seeded.items.some((item) => item.status === 'ACTIVE')).toBe(true);
    expect(seeded.items.some((item) => item.status === 'INACTIVE')).toBe(true);
    expect(contactFiltered.total).toBeGreaterThanOrEqual(1);
    expect(settlementFiltered.total).toBeGreaterThanOrEqual(1);
  });

  it('provides stable customer category seeds with status coverage and keyword filtering', () => {
    const seeded = getMockCustomerCategoryList({
      offset: 0,
      limit: 50,
    });
    const filtered = getMockCustomerCategoryList({
      keyword: 'vip',
      offset: 0,
      limit: 50,
    });

    expect(seeded.total).toBeGreaterThanOrEqual(6);
    expect(seeded.items.some((item) => item.status === 'ACTIVE')).toBe(true);
    expect(seeded.items.some((item) => item.status === 'INACTIVE')).toBe(true);
    expect(filtered.total).toBeGreaterThanOrEqual(1);
  });

  it('provides stable member level seeds with status coverage and keyword filtering', () => {
    const seeded = getMockMemberLevelList({
      offset: 0,
      limit: 50,
    });
    const filtered = getMockMemberLevelList({
      keyword: '企微',
      offset: 0,
      limit: 50,
    });

    expect(seeded.total).toBeGreaterThanOrEqual(6);
    expect(seeded.items.some((item) => item.status === 'ACTIVE')).toBe(true);
    expect(seeded.items.some((item) => item.status === 'INACTIVE')).toBe(true);
    expect(filtered.total).toBeGreaterThanOrEqual(1);
  });

  it('provides stable customer tag seeds with palette coverage and keyword filtering', () => {
    const seeded = getMockCustomerTagList({
      offset: 0,
      limit: 50,
    });
    const filtered = getMockCustomerTagList({
      keyword: 'gold',
      offset: 0,
      limit: 50,
    });

    expect(seeded.total).toBeGreaterThanOrEqual(6);
    expect(seeded.items.filter((item) => item.color).length).toBeGreaterThanOrEqual(5);
    expect(filtered.total).toBeGreaterThanOrEqual(1);
  });
});
