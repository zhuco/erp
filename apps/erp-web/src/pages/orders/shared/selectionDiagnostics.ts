const resolveBooleanFromEnv = (rawValue: string | undefined): boolean | null => {
  if (!rawValue) {
    return null;
  }
  const normalized = rawValue.trim().toLowerCase();
  if (!normalized) {
    return null;
  }
  if (normalized === '1' || normalized === 'true' || normalized === 'yes' || normalized === 'on') {
    return true;
  }
  if (normalized === '0' || normalized === 'false' || normalized === 'no' || normalized === 'off') {
    return false;
  }
  return null;
};

export const resolveSelectionDiagnosticsEnabled = () => {
  const envOverride = resolveBooleanFromEnv(import.meta.env.VITE_ENABLE_SELECTION_DIAGNOSTICS);
  if (envOverride != null) {
    return envOverride;
  }
  return import.meta.env.DEV || import.meta.env.MODE === 'test';
};
