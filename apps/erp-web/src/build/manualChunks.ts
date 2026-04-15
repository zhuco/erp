export const resolveManualChunk = (id: string) => {
  if (!id) {
    return undefined;
  }
  if (id.includes('node_modules')) {
    if (id.includes('react')) {
      return 'vendor-react';
    }
    if (id.includes('@tanstack')) {
      return 'vendor-tanstack';
    }
    return 'vendor';
  }
  return undefined;
};
