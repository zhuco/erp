const AG_GRID_LICENSE_PATTERNS = [
  'AG Grid Enterprise License',
  'License Key Not Found',
  'For Trial Use Only',
  'If you want to hide the watermark please email info@ag-grid.com for a trial license key.',
  'All AG Grid Enterprise features are unlocked for trial.',
] as const;

const isAgGridLicenseBannerMessage = (value: unknown) => {
  const normalized = String(value ?? '').trim();
  if (!normalized) {
    return false;
  }
  if (/^\*+$/.test(normalized)) {
    return true;
  }
  return AG_GRID_LICENSE_PATTERNS.some((pattern) => normalized.includes(pattern));
};

export const installAgGridLicenseLogFilter = () => {
  if (typeof window === 'undefined') {
    return;
  }

  const globalState = window as Window & {
    __ERP_AG_GRID_LICENSE_LOG_FILTER_INSTALLED__?: boolean;
  };
  if (globalState.__ERP_AG_GRID_LICENSE_LOG_FILTER_INSTALLED__) {
    return;
  }

  const originalConsoleError = console.error.bind(console);
  console.error = (...args: unknown[]) => {
    if (args.length > 0 && args.every((arg) => isAgGridLicenseBannerMessage(arg))) {
      return;
    }
    originalConsoleError(...args);
  };

  globalState.__ERP_AG_GRID_LICENSE_LOG_FILTER_INSTALLED__ = true;
};
