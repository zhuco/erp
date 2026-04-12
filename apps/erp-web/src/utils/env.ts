import { z } from 'zod';

const envSchema = z.object({
  VITE_API_BASE_URL: z.string().min(1),
  VITE_BUILD_VERSION: z.string().optional(),
  VITE_USE_MOCKS: z.string().optional(),
  VITE_ROUTER_MODE: z.enum(['browser', 'hash']).optional(),
  VITE_ROUTER_BASENAME: z.string().optional()
});

type Env = z.infer<typeof envSchema>;

const fallbackEnv: Env = {
  VITE_API_BASE_URL: 'http://localhost:8080',
  VITE_BUILD_VERSION: 'dev',
  VITE_USE_MOCKS: undefined,
  VITE_ROUTER_MODE: 'browser',
  VITE_ROUTER_BASENAME: '/'
};

const metaEnv =
  (
    import.meta as ImportMeta & {
      env?: Record<string, string | undefined>;
    }
  ).env ?? {};

const parsedEnv = envSchema.safeParse(metaEnv);
const env = parsedEnv.success ? parsedEnv.data : fallbackEnv;

export const apiBaseUrl = env.VITE_API_BASE_URL;
export const buildVersion = env.VITE_BUILD_VERSION ?? 'dev';
export const useMocks = env.VITE_USE_MOCKS === 'true';

const normalizeBasePath = (value: string) => {
  if (!value || value === '/') {
    return '/';
  }
  const withLeading = value.startsWith('/') ? value : `/${value}`;
  return withLeading.endsWith('/') ? withLeading.slice(0, -1) : withLeading;
};

export const routerMode = env.VITE_ROUTER_MODE ?? 'browser';
export const routerBasePath = normalizeBasePath(
  env.VITE_ROUTER_BASENAME ?? metaEnv.BASE_URL ?? '/'
);
