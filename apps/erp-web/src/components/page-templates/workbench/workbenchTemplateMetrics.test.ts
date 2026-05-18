import { describe, expect, it } from 'vitest';

import {
  calculateTemplateAdoptionRate,
  classifyTemplateUsage,
  isTemplateAdoptionPassing,
} from './workbenchTemplateMetrics';

describe('workbench template metrics', () => {
  it('calculates adoption rate against applicable pages only', () => {
    const result = calculateTemplateAdoptionRate([
      { file: 'src/pages/orders/OrderReviewPage.tsx', applicable: true, hasTemplate: true },
      { file: 'src/pages/orders/OrderQueryPage.tsx', applicable: true, hasTemplate: false },
      { file: 'src/pages/orders/components/OrderReviewPanels.tsx', applicable: false, hasTemplate: false },
    ]);

    expect(result.applicableTotal).toBe(2);
    expect(result.templateTotal).toBe(1);
    expect(result.rate).toBe(0.5);
  });

  it('passes only when adoption is at least 95 percent', () => {
    expect(isTemplateAdoptionPassing(0.95)).toBe(true);
    expect(isTemplateAdoptionPassing(0.949)).toBe(false);
  });

  it('classifies allowed templates and legacy tokens', () => {
    expect(classifyTemplateUsage('<WorkbenchPageTemplate />')).toMatchObject({
      hasTemplate: true,
      hasLegacyTemplate: false,
    });
    expect(classifyTemplateUsage('<OrderReviewWorkbenchTemplate />')).toMatchObject({
      hasTemplate: false,
      hasLegacyTemplate: true,
    });
  });
});
