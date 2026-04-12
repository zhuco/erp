import type { PrintTemplate } from '@/api/printTemplates';

export type PrintTemplatePreference = {
  template_id: string;
};

export type PrintTemplateOption = {
  template_id: string;
  name: string;
  source: 'builtin' | 'cloud';
  version?: number;
};

const storageKey = 'erp.print.template.default';

export const buildTemplateKey = (template_id: string) => template_id;

export const parseTemplateKey = (value?: string): PrintTemplatePreference | null => {
  if (!value) {
    return null;
  }
  const template_id = value;
  if (!template_id) {
    return null;
  }
  return { template_id };
};

export const getStoredPrintTemplate = (): PrintTemplatePreference | null => {
  if (typeof window === 'undefined') {
    return null;
  }
  const raw = window.localStorage.getItem(storageKey);
  if (!raw) {
    return null;
  }
  try {
    const parsed = JSON.parse(raw) as PrintTemplatePreference;
    if (parsed?.template_id) {
      return parsed;
    }
  } catch {
    return null;
  }
  return null;
};

export const setStoredPrintTemplate = (template: PrintTemplatePreference | null) => {
  if (typeof window === 'undefined') {
    return;
  }
  if (!template) {
    window.localStorage.removeItem(storageKey);
    return;
  }
  window.localStorage.setItem(storageKey, JSON.stringify(template));
};

export const builtinPrintTemplateOptions: PrintTemplateOption[] = [
  {
    template_id: 'builtin-shipping-label-v1',
    name: '内置面单模板 V1',
    source: 'builtin'
  },
  {
    template_id: 'builtin-shipping-label-v2',
    name: '内置面单模板 V2',
    source: 'builtin'
  }
];

export const mapPrintTemplateToOption = (template: PrintTemplate): PrintTemplateOption => ({
  template_id: template.template_id,
  name: `${template.template_type} v${template.version}`,
  source: 'cloud',
  version: template.version
});
