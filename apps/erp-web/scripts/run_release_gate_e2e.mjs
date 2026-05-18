import net from 'node:net';
import { spawnSync } from 'node:child_process';

const HOST = process.env.PW_E2E_HOST ?? '127.0.0.1';
const SPECS = [
  'e2e/system-governance.spec.ts',
  'e2e/task8-stats-observability-sso.spec.ts',
  'e2e/release-gate-main-chain.spec.ts',
  'e2e/release-gate-cross-chain.spec.ts',
  'e2e/workbench-a11y.spec.ts',
  'e2e/workbench-layout-presets.spec.ts',
  'e2e/order-workbench-pages.spec.ts',
];

function findFreePort(host) {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.once('error', reject);
    server.listen(0, host, () => {
      const address = server.address();
      if (!address || typeof address === 'string') {
        server.close(() => reject(new Error('无法获取可用端口')));
        return;
      }
      const port = address.port;
      server.close((closeError) => {
        if (closeError) {
          reject(closeError);
          return;
        }
        resolve(port);
      });
    });
  });
}

async function main() {
  const selectedPort = process.env.PW_E2E_PORT
    ? Number(process.env.PW_E2E_PORT)
    : await findFreePort(HOST);

  if (!Number.isInteger(selectedPort) || selectedPort <= 0) {
    throw new Error(`非法端口: ${selectedPort}`);
  }

  const result = spawnSync(
    'pnpm',
    ['exec', 'playwright', 'test', '--workers=1', '--retries=1', ...SPECS],
    {
      stdio: 'inherit',
      env: {
        ...process.env,
        PW_E2E_HOST: HOST,
        PW_E2E_PORT: String(selectedPort),
      },
      shell: process.platform === 'win32',
    }
  );

  if (result.error) {
    throw result.error;
  }
  process.exit(result.status ?? 1);
}

main().catch((error) => {
  console.error(`[release-gate-e2e] 执行失败: ${error?.message ?? error}`);
  process.exit(1);
});
