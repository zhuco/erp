const REPLACEMENT_CHAR = '\uFFFD';
const CJK_CHAR_RE = /[\u3400-\u9FFF]/;
const QUESTION_MARK_RUN_RE = /\?{2,}/;
const LATIN_MOJIBAKE_RE = /[\u00C3\u00D0\u00D1\u00E6\u00E5\u00E4]/;

export const isLikelyMojibake = (raw: string): boolean => {
  const value = String(raw ?? '').trim();
  if (!value) return false;
  if (value.includes(REPLACEMENT_CHAR) || value.includes('\u951F')) return true;
  if (QUESTION_MARK_RUN_RE.test(value)) return true;
  if (!CJK_CHAR_RE.test(value) && LATIN_MOJIBAKE_RE.test(value)) return true;
  return false;
};

export const resolveCourierOptionValue = (name: string, code: string): string => {
  const normalizedName = String(name ?? '').trim();
  const normalizedCode = String(code ?? '').trim().toUpperCase();

  if (!normalizedName) {
    return normalizedCode;
  }
  if (isLikelyMojibake(normalizedName) && normalizedCode) {
    return normalizedCode;
  }
  return normalizedName;
};

export const buildCourierOptionLabel = (name: string, code: string): string => {
  const normalizedName = String(name ?? '').trim();
  const normalizedCode = String(code ?? '').trim().toUpperCase();

  if (!normalizedName) {
    return normalizedCode || '-';
  }
  if (!normalizedCode) {
    return isLikelyMojibake(normalizedName) ? 'Unknown Courier' : normalizedName;
  }
  if (isLikelyMojibake(normalizedName)) {
    return normalizedCode;
  }
  return `${normalizedName} (${normalizedCode})`;
};
