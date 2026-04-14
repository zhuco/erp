import { describe, expect, it } from 'vitest';

import { buildCourierOptionLabel, resolveCourierOptionValue } from './courierDisplay';

describe('courierDisplay', () => {
  it('uses courier code as label for mojibake-like courier names', () => {
    expect(buildCourierOptionLabel('????', 'SF')).toBe('SF');
  });

  it('uses readable name and code for normal courier values', () => {
    expect(buildCourierOptionLabel('SF Express', 'SF')).toBe('SF Express (SF)');
  });

  it('uses code as value when courier name is mojibake-like', () => {
    expect(resolveCourierOptionValue('????', 'YTO')).toBe('YTO');
  });

  it('keeps courier name as value when name is readable', () => {
    expect(resolveCourierOptionValue('ZTO Express', 'ZTO')).toBe('ZTO Express');
  });
});
