import { describe, expect, it } from 'vitest';

import { normalizeVisibleText } from './visibleText';

const escapedNovelProductionWork = String.raw`\u5c0f\u8bf4\u751f\u4ea7\u5de5\u4f5c`;
const novelProductionWork = String.fromCharCode(0x5c0f, 0x8bf4, 0x751f, 0x4ea7, 0x5de5, 0x4f5c);
const ordinaryChineseText = String.fromCharCode(0x666e, 0x901a, 0x4e2d, 0x6587);
const escapedFullWidthColon = String.raw`\uff1a`;
const fullWidthColon = String.fromCharCode(0xff1a);
const titleText = String.fromCharCode(0x6807, 0x9898);
const escapedBomMarker = `${String.raw`\uFEFF`}${titleText}`;
const escapedReplacementMarker = `${String.raw`\uFFFD`}${titleText}`;
const technicalCommonJsPrefix = String.raw`\u0000commonjsHelpers.js`;

describe('normalizeVisibleText', () => {
  it('decodes escaped CJK Unicode text before rendering', () => {
    expect(normalizeVisibleText(escapedNovelProductionWork)).toBe(novelProductionWork);
    expect(normalizeVisibleText(escapedFullWidthColon)).toBe(fullWidthColon);
  });

  it('keeps ordinary Chinese text unchanged', () => {
    expect(normalizeVisibleText(ordinaryChineseText)).toBe(ordinaryChineseText);
  });

  it('does not decode non-display control escapes', () => {
    expect(normalizeVisibleText(technicalCommonJsPrefix)).toBe(technicalCommonJsPrefix);
    expect(normalizeVisibleText(escapedBomMarker)).toBe(escapedBomMarker);
    expect(normalizeVisibleText(escapedReplacementMarker)).toBe(escapedReplacementMarker);
  });
});
