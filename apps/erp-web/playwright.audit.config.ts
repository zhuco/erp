import path from 'node:path';
import { fileURLToPath } from 'node:url';

import baseConfig from './playwright.config';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const auditOutputDir = path.resolve(__dirname, '.playwright-audit/test-results');

export default {
  ...baseConfig,
  outputDir: auditOutputDir,
  reporter: [['list']],
};
