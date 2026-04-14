import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { bundleOpenApiDocument } from './openapi/lib/openapi_tools.mjs';

const currentFilePath = fileURLToPath(import.meta.url);
const scriptDir = path.dirname(currentFilePath);
const repoRoot = path.resolve(scriptDir, '..');

const inputPath = path.join(repoRoot, 'openapi', 'src', 'openapi.yaml');
const outputPath = path.join(repoRoot, 'openapi', 'erp_api_v1.yaml');

bundleOpenApiDocument({ inputPath, outputPath });
console.log(`[openapi-bundle] Bundled ${inputPath} -> ${outputPath}`);
