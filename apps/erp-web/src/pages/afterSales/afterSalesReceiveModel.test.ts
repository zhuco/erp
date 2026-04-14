import { describe, expect, it } from 'vitest';

import { buildReceiveModel, NO_WAREHOUSE_HINT } from './afterSalesReceiveModel';

const defaultWarehouse = {
  warehouse_id: 'default-1',
  name: 'Default Warehouse',
  is_default: true
};

const fallbackWarehouse = {
  warehouse_id: 'fallback-1',
  name: 'Fallback Warehouse',
  is_default: false
};

describe('after sales receive model', () => {
  it('automatically picks the lone warehouse', () => {
    const model = buildReceiveModel({ warehouses: [fallbackWarehouse], returnReceiptStatus: 'OPEN' });

    expect(model.initialWarehouseId).toBe(fallbackWarehouse.warehouse_id);
    expect(model.canSubmit).toBe(true);
    expect(model.submitDisabledHint).toBeUndefined();
    expect(model.modeLabel).toBe('正常收货');
    expect(model.modeDescription).toContain('正常');
  });

  it('prefers the default warehouse when there are multiple', () => {
    const model = buildReceiveModel({ warehouses: [fallbackWarehouse, defaultWarehouse], returnReceiptStatus: 'APPROVED' });

    expect(model.initialWarehouseId).toBe(defaultWarehouse.warehouse_id);
    expect(model.canSubmit).toBe(true);
    expect(model.submitDisabledHint).toBeUndefined();
  });

  it('blocks submission when no warehouses are available', () => {
    const model = buildReceiveModel({ warehouses: [], returnReceiptStatus: 'OPEN' });

    expect(model.initialWarehouseId).toBeUndefined();
    expect(model.canSubmit).toBe(false);
    expect(model.submitDisabledHint).toBe(NO_WAREHOUSE_HINT);
  });

  it('describes redo mode when the receipt was cancelled', () => {
    const model = buildReceiveModel({ warehouses: [defaultWarehouse], returnReceiptStatus: 'CANCELLED' });

    expect(model.modeLabel).toBe('重做收货');
    expect(model.modeDescription).toContain('取消');
  });
});
