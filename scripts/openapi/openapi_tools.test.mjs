import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

import {
  bundleOpenApiDocument,
  validateOpenApiGate,
} from './lib/openapi_tools.mjs';

function writeFile(filePath, contents) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, contents, 'utf8');
}

test('bundleOpenApiDocument inlines top-level external refs into bundled output', () => {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'openapi-bundle-'));
  const inputPath = path.join(tmpDir, 'openapi', 'src', 'openapi.yaml');
  const outputPath = path.join(tmpDir, 'openapi', 'erp_api_v1.yaml');

  writeFile(
    inputPath,
    [
      'openapi: 3.0.3',
      'info:',
      '  title: ERP API',
      '  version: 1.0.0',
      'servers:',
      '  - url: http://localhost:8080',
      'paths:',
      "  $ref: './paths.yaml'",
      'components:',
      "  $ref: './components.yaml'",
      '',
    ].join('\n'),
  );

  writeFile(
    path.join(path.dirname(inputPath), 'paths.yaml'),
    [
      '/healthz:',
      '  get:',
      '    responses:',
      '      "200":',
      '        description: ok',
      '/auth/login:',
      '  post:',
      '    requestBody:',
      '      required: true',
      '      content:',
      '        application/json:',
      '          schema:',
      '            $ref: "#/components/schemas/LoginRequest"',
      '    responses:',
      '      "200":',
      '        description: token',
      '        content:',
      '          application/json:',
      '            schema:',
      '              $ref: "#/components/schemas/LoginResponse"',
      '',
    ].join('\n'),
  );

  writeFile(
    path.join(path.dirname(inputPath), 'components.yaml'),
    [
      'securitySchemes:',
      '  bearerAuth:',
      '    type: http',
      '    scheme: bearer',
      'schemas:',
      '  LoginRequest:',
      '    type: object',
      '    required: [tenant_id, email, password]',
      '    properties:',
      '      tenant_id: { type: string }',
      '      email: { type: string }',
      '      password: { type: string }',
      '  LoginResponse:',
      '    type: object',
      '    required: [access_token]',
      '    properties:',
      '      access_token: { type: string }',
      '',
    ].join('\n'),
  );

  const bundled = bundleOpenApiDocument({ inputPath, outputPath });

  assert.ok(fs.existsSync(outputPath));
  assert.equal(bundled.openapi, '3.0.3');
  assert.deepEqual(Object.keys(bundled.paths).sort(), ['/auth/login', '/healthz']);
  assert.equal(bundled.components.schemas.LoginRequest.type, 'object');
  assert.equal(
    bundled.paths['/auth/login'].post.responses['200'].content['application/json'].schema.$ref,
    '#/components/schemas/LoginResponse',
  );
});

test('validateOpenApiGate checks permissions responses and persistence semantically', () => {
  const openapiDocument = {
    openapi: '3.0.3',
    paths: {
      '/v1/tenants/{tenant_id}/shop-products/download': {
        post: {
          'x-permissions': ['shop_products.write'],
          'x-persistence-required': true,
          'x-persistence-tables': ['shop_product_tasks', 'shop_products', 'shop_product_skus'],
          responses: {
            '200': {},
            '400': {},
            '401': {},
            '403': {},
          },
        },
      },
    },
  };

  assert.doesNotThrow(() => {
    validateOpenApiGate({
      openapiDocument,
      checks: [
        {
          pathKey: '/v1/tenants/{tenant_id}/shop-products/download',
          method: 'post',
          requiredPermissions: ['shop_products.write'],
          requiredStatusCodes: ['200', '400', '401', '403'],
          requiredPersistenceTables: ['shop_product_tasks', 'shop_products', 'shop_product_skus'],
        },
      ],
    });
  });

  assert.throws(() => {
    validateOpenApiGate({
      openapiDocument: {
        openapi: '3.0.3',
        paths: {
          '/v1/tenants/{tenant_id}/shop-products/download': {
            post: {
              'x-permissions': ['shop_products.write'],
              responses: {
                '200': {},
                '400': {},
                '401': {},
                '403': {},
              },
            },
          },
        },
      },
      checks: [
        {
          pathKey: '/v1/tenants/{tenant_id}/shop-products/download',
          method: 'post',
          requiredPermissions: ['shop_products.write'],
          requiredStatusCodes: ['200', '400', '401', '403'],
          requiredPersistenceTables: ['shop_product_tasks'],
        },
      ],
    });
  }, /missing persistence declaration/i);
});
