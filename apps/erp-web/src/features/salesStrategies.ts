export type SalesStrategyCondition = {
  field: string;
  operator: string;
  value: string;
};

export type SalesStrategyMatchRule = {
  scope?: string;
  areas?: string[];
};

export type SalesStrategy = {
  id: string;
  type: string;
  name: string;
  priority: number;
  remark?: string;
  salesChannel?: string[];
  salesOrderType?: string;
  salesOrderTag?: string;
  commissionMatchMode?: string;
  commissionAssignee?: string;
  commissionNotePrefix?: string;
  commissionNoteContent?: string;
  commissionNoteSuffix?: string;
  commissionFallbackAssignee?: string;
  commissionExceptionTag?: string;
  parseContent?: string;
  rulePrefix?: string;
  ruleSeparator?: string;
  ruleSuffix?: string;
  priceBasis?: string;
  priceDiscount?: number;
  parseErrorTag?: string;
  parseSuccessTag?: string;
  autoExecute?: boolean;
  splitCount?: number;
  mergeCount?: number;
  logisticsMatchMode?: string;
  logisticsIncludeRules?: SalesStrategyMatchRule[];
  logisticsExcludeRules?: SalesStrategyMatchRule[];
  logisticsValidFrom?: string;
  logisticsValidTo?: string;
  logisticsSmartSelect?: boolean;
  warehouseMatchMode?: string;
  warehouseIncludeRules?: SalesStrategyMatchRule[];
  warehouseExcludeRules?: SalesStrategyMatchRule[];
  warehouseValidFrom?: string;
  warehouseValidTo?: string;
  warehouseSmartSelect?: boolean;
  executionStrategy?: string;
  executeTime?: string;
  validPeriod: string;
  enabled: boolean;
  conditions: SalesStrategyCondition[];
  exclusions: SalesStrategyCondition[];
  updatedAt: string;
};

export const salesStrategyStorageKey = 'sales.strategy.settings';
export const salesStrategyUpdatedEventName = 'sales-strategy-settings-updated';

export const loadSalesStrategies = (): SalesStrategy[] => {
  if (typeof localStorage === 'undefined') {
    return [];
  }
  const raw = localStorage.getItem(salesStrategyStorageKey);
  if (!raw) {
    return [];
  }
  try {
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? (parsed as SalesStrategy[]) : [];
  } catch {
    return [];
  }
};

export const saveSalesStrategies = (strategies: SalesStrategy[]) => {
  if (typeof localStorage === 'undefined') {
    return;
  }
  localStorage.setItem(salesStrategyStorageKey, JSON.stringify(strategies));
  if (typeof window !== 'undefined') {
    window.dispatchEvent(new CustomEvent(salesStrategyUpdatedEventName));
  }
};
