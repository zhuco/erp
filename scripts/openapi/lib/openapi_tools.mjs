import fs from 'node:fs';
import path from 'node:path';
import YAML from 'yaml';

function isPlainObject(value) {
  return value !== null && typeof value === 'object' && !Array.isArray(value);
}

function cloneValue(value) {
  return globalThis.structuredClone ? globalThis.structuredClone(value) : JSON.parse(JSON.stringify(value));
}

function detectLineEnding(filePath) {
  if (!fs.existsSync(filePath)) {
    return '\n';
  }

  const text = fs.readFileSync(filePath, 'utf8');
  return text.includes('\r\n') ? '\r\n' : '\n';
}

function readYamlFile(filePath) {
  if (!fs.existsSync(filePath)) {
    throw new Error(`missing OpenAPI file: ${filePath}`);
  }

  return YAML.parse(fs.readFileSync(filePath, 'utf8'));
}

function splitRef(ref) {
  const [filePart, fragment = ''] = ref.split('#');
  return {
    filePart,
    fragment,
  };
}

function resolvePointer(document, fragment, ref) {
  if (!fragment) {
    return document;
  }

  if (!fragment.startsWith('/')) {
    throw new Error(`unsupported OpenAPI ref fragment in ${ref}`);
  }

  const segments = fragment
    .split('/')
    .slice(1)
    .map((segment) => segment.replace(/~1/g, '/').replace(/~0/g, '~'));

  let current = document;
  for (const segment of segments) {
    if (!isPlainObject(current) && !Array.isArray(current)) {
      throw new Error(`unable to resolve OpenAPI ref ${ref}`);
    }

    current = current[segment];
    if (current === undefined) {
      throw new Error(`unable to resolve OpenAPI ref ${ref}`);
    }
  }

  return current;
}

function resolveExternalRefs(value, baseDir, seenRefs = new Set()) {
  if (Array.isArray(value)) {
    return value.map((entry) => resolveExternalRefs(entry, baseDir, seenRefs));
  }

  if (!isPlainObject(value)) {
    return value;
  }

  const ref = value.$ref;
  if (typeof ref === 'string' && !ref.startsWith('#')) {
    const { filePart, fragment } = splitRef(ref);
    const resolvedPath = path.resolve(baseDir, filePart);
    const refKey = `${resolvedPath}#${fragment}`;
    if (seenRefs.has(refKey)) {
      throw new Error(`circular OpenAPI ref detected: ${ref}`);
    }

    const referencedDocument = readYamlFile(resolvedPath);
    const referencedValue = resolvePointer(referencedDocument, fragment, ref);
    return resolveExternalRefs(cloneValue(referencedValue), path.dirname(resolvedPath), new Set([...seenRefs, refKey]));
  }

  const resolvedObject = {};
  for (const [key, entry] of Object.entries(value)) {
    resolvedObject[key] = resolveExternalRefs(entry, baseDir, seenRefs);
  }

  return resolvedObject;
}

export function loadOpenApiDocument(filePath) {
  return readYamlFile(filePath);
}

function indentYamlBlock(text, indent = '  ', eol = '\n') {
  return text
    .replace(/\r\n/g, '\n')
    .trimEnd()
    .split('\n')
    .map((line) => (line.length > 0 ? `${indent}${line}` : ''))
    .join(eol);
}

function tryBundleTopLevelRefs({ inputPath, openapiDocument, eol }) {
  const pathRef = openapiDocument?.paths?.$ref;
  const componentsRef = openapiDocument?.components?.$ref;
  const pathKeys = isPlainObject(openapiDocument?.paths) ? Object.keys(openapiDocument.paths) : [];
  const componentKeys = isPlainObject(openapiDocument?.components) ? Object.keys(openapiDocument.components) : [];

  if (pathKeys.length !== 1 || componentKeys.length !== 1 || typeof pathRef !== 'string' || typeof componentsRef !== 'string') {
    return null;
  }

  if (pathRef.startsWith('#') || componentsRef.startsWith('#')) {
    return null;
  }

  const sourceLines = fs.readFileSync(inputPath, 'utf8').replace(/\r\n/g, '\n').split('\n');
  const pathsIndex = sourceLines.findIndex((line) => line === 'paths:');
  const componentsIndex = sourceLines.findIndex((line) => line === 'components:');
  if (pathsIndex === -1 || componentsIndex === -1 || componentsIndex <= pathsIndex) {
    return null;
  }

  const headerLines = sourceLines.slice(0, pathsIndex);
  const headerText = headerLines.join('\n').trimEnd();
  const pathsText = fs.readFileSync(path.resolve(path.dirname(inputPath), splitRef(pathRef).filePart), 'utf8');
  const componentsText = fs.readFileSync(path.resolve(path.dirname(inputPath), splitRef(componentsRef).filePart), 'utf8');

  return [
    headerText,
    'paths:',
    indentYamlBlock(pathsText, '  ', eol),
    'components:',
    indentYamlBlock(componentsText, '  ', eol),
    '',
  ].join(eol);
}

export function bundleOpenApiDocument({ inputPath, outputPath }) {
  const openapiDocument = loadOpenApiDocument(inputPath);
  const bundledDocument = resolveExternalRefs(openapiDocument, path.dirname(inputPath));
  const eol = detectLineEnding(outputPath);
  const bundledText = (
    tryBundleTopLevelRefs({ inputPath, openapiDocument, eol }) ?? YAML.stringify(bundledDocument)
  ).replace(/\n/g, eol);

  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, bundledText, 'utf8');

  return bundledDocument;
}

function getMethodDocument(openapiDocument, pathKey, method) {
  const pathDocument = openapiDocument?.paths?.[pathKey];
  if (!pathDocument) {
    throw new Error(`missing OpenAPI path: ${pathKey}`);
  }

  const methodDocument = pathDocument[method];
  if (!methodDocument) {
    throw new Error(`path ${pathKey} missing ${method.toUpperCase()} method`);
  }

  return methodDocument;
}

function ensurePermissions(methodDocument, requiredPermissions, context) {
  const permissions = Array.isArray(methodDocument['x-permissions']) ? methodDocument['x-permissions'] : [];
  for (const permission of requiredPermissions) {
    if (!permissions.includes(permission)) {
      throw new Error(`${context} missing permission declaration: ${permission}`);
    }
  }
}

function ensureResponses(methodDocument, requiredStatusCodes, context) {
  const responses = isPlainObject(methodDocument.responses) ? methodDocument.responses : {};
  for (const statusCode of requiredStatusCodes) {
    if (!(statusCode in responses)) {
      throw new Error(`${context} missing response status: ${statusCode}`);
    }
  }
}

function ensurePersistence(methodDocument, requiredPersistenceTables, context) {
  if (methodDocument['x-persistence-required'] !== true) {
    throw new Error(`${context} missing persistence declaration: x-persistence-required`);
  }

  const tables = Array.isArray(methodDocument['x-persistence-tables']) ? methodDocument['x-persistence-tables'] : [];
  for (const table of requiredPersistenceTables) {
    if (!tables.includes(table)) {
      throw new Error(`${context} missing persistence declaration: ${table}`);
    }
  }
}

export function validateOpenApiGate({ openapiDocument, checks }) {
  for (const check of checks) {
    const methodDocument = getMethodDocument(openapiDocument, check.pathKey, check.method);
    const context = `${check.method.toUpperCase()} ${check.pathKey}`;
    ensurePermissions(methodDocument, check.requiredPermissions ?? [], context);
    ensureResponses(methodDocument, check.requiredStatusCodes ?? [], context);

    if (check.requiredPersistenceTables?.length) {
      ensurePersistence(methodDocument, check.requiredPersistenceTables, context);
    }
  }
}
