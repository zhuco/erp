const deviceKey = 'erp.device_id';

const generateDeviceId = (): string => {
  if (typeof crypto !== 'undefined' && 'randomUUID' in crypto) {
    return crypto.randomUUID();
  }
  return `device-${Date.now()}-${Math.random().toString(16).slice(2)}`;
};

export const getDeviceFingerprint = (): string => {
  if (typeof window === 'undefined') {
    return 'unknown-device';
  }
  try {
    const stored = window.localStorage.getItem(deviceKey);
    if (stored) {
      return stored;
    }
    const next = generateDeviceId();
    window.localStorage.setItem(deviceKey, next);
    return next;
  } catch {
    return generateDeviceId();
  }
};
