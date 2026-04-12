import { useMocks } from '@/utils/env';

import type { PrintShippingLabelPayload } from './types';

export type PrintBridgeResult<T = undefined> =
  | { success: true; data?: T }
  | { success: false; error: string };

export type PrintBridgeInfo = {
  provider: string;
  version?: string;
};

export type PrintBridgePrinter = {
  id: string;
  name: string;
  is_default?: boolean;
};

export type PrintBridge = {
  detect: () => Promise<PrintBridgeResult<PrintBridgeInfo>>;
  printShippingLabel: (
    payload: PrintShippingLabelPayload
  ) => Promise<PrintBridgeResult>;
  getPrinters?: () => Promise<PrintBridgeResult<PrintBridgePrinter[]>>;
};

type PlatformPrintApi = {
  provider?: string;
  version?: string;
  printShippingLabel?: (payload: PrintShippingLabelPayload) => Promise<void> | void;
  getPrinters?: () => Promise<PrintBridgePrinter[]> | PrintBridgePrinter[];
};

const resolvePlatformApi = (): PlatformPrintApi | null => {
  if (typeof window === 'undefined') {
    return null;
  }
  const win = window as typeof window & {
    PddPrint?: PlatformPrintApi;
    PlatformPrint?: PlatformPrintApi;
  };
  return win.PddPrint ?? win.PlatformPrint ?? null;
};

const platformBridge: PrintBridge = {
  detect: async () => {
    const api = resolvePlatformApi();
    if (!api || typeof api.printShippingLabel !== 'function') {
      return { success: false, error: '平台打印组件未就绪' };
    }
    return {
      success: true,
      data: {
        provider: api.provider ?? 'platform',
        version: api.version
      }
    };
  },
  printShippingLabel: async (payload) => {
    const api = resolvePlatformApi();
    if (!api || typeof api.printShippingLabel !== 'function') {
      return { success: false, error: '平台打印组件未安装' };
    }
    try {
      const result = api.printShippingLabel(payload);
      if (result instanceof Promise) {
        await result;
      }
      return { success: true };
    } catch (err) {
      return {
        success: false,
        error: err instanceof Error ? err.message : '打印失败'
      };
    }
  },
  getPrinters: async () => {
    const api = resolvePlatformApi();
    if (!api || typeof api.getPrinters !== 'function') {
      return { success: false, error: '平台打印组件未提供打印机列表' };
    }
    try {
      const result = api.getPrinters();
      const printers = result instanceof Promise ? await result : result;
      return { success: true, data: printers };
    } catch (err) {
      return {
        success: false,
        error: err instanceof Error ? err.message : '获取打印机失败'
      };
    }
  }
};

const mockBridge: PrintBridge = {
  detect: async () => ({
    success: true,
    data: {
      provider: 'mock',
      version: '0.1'
    }
  }),
  printShippingLabel: async () => ({ success: true }),
  getPrinters: async () => ({
    success: true,
    data: [
      {
        id: 'mock-printer',
        name: 'Mock Printer',
        is_default: true
      }
    ]
  })
};

export const printBridge: PrintBridge = useMocks ? mockBridge : platformBridge;
