import dayjs from 'dayjs';
import { describe, expect, it } from 'vitest';

import {
  createMockSkuBatch,
  getMockSkuBatchList,
  updateMockSkuBatch,
} from './skus';

describe('sku batch runtime governance mocks', () => {
  it('persists production_date when creating and updating food batches', () => {
    const batchNo = `ROUND12-FOOD-${Date.now()}`;
    const productionDate = dayjs().subtract(120, 'day').startOf('day').toISOString();
    const expireAt = dayjs(productionDate).add(180, 'day').toISOString();

    const created = createMockSkuBatch({
      sku_id: 'SKU-FOOD-GOV-001',
      batch_no: batchNo,
      expire_at: expireAt,
      production_date: productionDate,
    } as Parameters<typeof createMockSkuBatch>[0] & { production_date: string });

    expect((created as { production_date?: string | null }).production_date).toBe(productionDate);

    const listed = getMockSkuBatchList({
      sku_id: 'SKU-FOOD-GOV-001',
      limit: 200,
    } as Parameters<typeof getMockSkuBatchList>[0]);
    const createdRow = listed.data.find((item) => item.batch_no === batchNo);
    expect((createdRow as { production_date?: string | null } | undefined)?.production_date).toBe(
      productionDate,
    );

    const nextProductionDate = dayjs(productionDate).add(2, 'day').toISOString();
    const updated = updateMockSkuBatch(
      created.batch_id,
      {
        production_date: nextProductionDate,
      } as Parameters<typeof updateMockSkuBatch>[1] & { production_date: string },
    );

    expect((updated as { production_date?: string | null }).production_date).toBe(
      nextProductionDate,
    );
  });
});
