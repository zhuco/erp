import { readFileSync, readdirSync } from 'node:fs';
import { relative, resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

const sourceRoot = resolve(__dirname, '../..');
const ignoredFiles = new Set([
  'app/build/manualChunks.ts',
  'app/build/manualChunks.test.ts',
  'app/layout/visibleText.test.ts',
  'app/layout/visibleText.ts',
]);
const fileExtensions = new Set(['.ts', '.tsx']);
const cjkUnicodeEscapePattern = /\\u([0-9a-fA-F]{4})/g;
const allowedTechnicalEscapes = new Set(['feff', 'fffd', '951f']);
const allowedTechnicalPatterns = [/\[[^\]]*\\u[0-9a-fA-F]{4}-\\u[0-9a-fA-F]{4}[^\]]*\]/];
const visibleTextRanges: Array<readonly [number, number]> = [
  [0x3000, 0x303f],
  [0x3400, 0x4dbf],
  [0x4e00, 0x9fff],
  [0xf900, 0xfaff],
  [0xff00, 0xffef],
];

const collectSourceFiles = (directory: string): string[] =>
  readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    const file = resolve(directory, entry.name);
    if (entry.isDirectory()) {
      return collectSourceFiles(file);
    }
    if (!entry.isFile()) {
      return [];
    }
    const extension = entry.name.slice(entry.name.lastIndexOf('.'));
    return fileExtensions.has(extension) ? [file] : [];
  });

const toRelativeSourcePath = (file: string) => relative(sourceRoot, file).replace(/\\/g, '/');

const isVisibleTextEscape = (hex: string) => {
  const codePoint = Number.parseInt(hex, 16);
  return visibleTextRanges.some(([start, end]) => codePoint >= start && codePoint <= end);
};

describe('visible text encoding', () => {
  it('does not render CJK text as literal Unicode escape sequences', () => {
    const offenders = collectSourceFiles(sourceRoot).flatMap((file) => {
      const relativePath = toRelativeSourcePath(file);
      if (ignoredFiles.has(relativePath)) {
        return [];
      }

      return readFileSync(file, 'utf8')
        .split(/\r?\n/)
        .flatMap((line, index) => {
          const matches = Array.from(line.matchAll(cjkUnicodeEscapePattern)).filter((match) => {
            const hex = match[1].toLowerCase();
            return (
              !allowedTechnicalEscapes.has(hex) &&
              isVisibleTextEscape(hex) &&
              !allowedTechnicalPatterns.some((pattern) => pattern.test(line))
            );
          });

          return matches.map((match) => `${relativePath}:${index + 1} ${match[0]}`);
        });
    });

    expect(offenders).toEqual([]);
  });
});
