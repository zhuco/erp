const unicodeEscapePattern = /\\u([0-9a-fA-F]{4})/g;

const visibleTextRanges: Array<readonly [number, number]> = [
  [0x3000, 0x303f],
  [0x3400, 0x4dbf],
  [0x4e00, 0x9fff],
  [0xf900, 0xfaff],
  [0xff00, 0xffef],
];

const isVisibleCjkCodePoint = (codePoint: number) =>
  visibleTextRanges.some(([start, end]) => codePoint >= start && codePoint <= end);

export const normalizeVisibleText = (value: string): string =>
  value.replace(unicodeEscapePattern, (match, hex: string) => {
    const codePoint = Number.parseInt(hex, 16);
    return isVisibleCjkCodePoint(codePoint) ? String.fromCharCode(codePoint) : match;
  });
