import type { ReturnReceipt } from '@/api/afterSalesOps';

export type ReceiveModelWarehouse = {
  warehouse_id: string;
  name: string;
  is_default: boolean;
};

export const NO_WAREHOUSE_HINT = '当前租户暂无可用仓库，提交前请先创建仓库或联系管理员。';

export const selectInitialWarehouse = (warehouses: ReceiveModelWarehouse[]): string | undefined => {
  if (warehouses.length === 0) {
    return undefined;
  }
  if (warehouses.length === 1) {
    return warehouses[0].warehouse_id;
  }
  const defaultWarehouse = warehouses.find((warehouse) => warehouse.is_default === true);
  return defaultWarehouse?.warehouse_id ?? warehouses[0].warehouse_id;
};

export type ReceiveMode = 'NORMAL' | 'REDO';

export type ReceiveModel = {
  initialWarehouseId?: string;
  canSubmit: boolean;
  submitDisabledHint?: string;
  modeLabel: string;
  modeDescription: string;
};

type BuildReceiveModelInput = {
  warehouses: ReceiveModelWarehouse[];
  returnReceiptStatus?: ReturnReceipt['status'];
};

const MODE_CONFIG: Record<ReceiveMode, { label: string; description: string }> = {
  NORMAL: {
    label: '正常收货',
    description: '正常收货：该单号未重做收货，请确认仓库后提交。'
  },
  REDO: {
    label: '重做收货',
    description: '重做收货：该收货曾被取消，当前为重做收货，请再次确认仓库再提交。'
  }
};

export const buildReceiveModel = ({ warehouses, returnReceiptStatus }: BuildReceiveModelInput): ReceiveModel => {
  const initialWarehouseId = selectInitialWarehouse(warehouses);
  const mode: ReceiveMode = returnReceiptStatus === 'CANCELLED' ? 'REDO' : 'NORMAL';
  const canSubmit = warehouses.length > 0;
  return {
    initialWarehouseId,
    canSubmit,
    submitDisabledHint: canSubmit ? undefined : NO_WAREHOUSE_HINT,
    modeLabel: MODE_CONFIG[mode].label,
    modeDescription: MODE_CONFIG[mode].description
  };
};
