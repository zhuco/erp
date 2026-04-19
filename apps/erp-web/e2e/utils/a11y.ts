import { expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';

type WorkbenchA11yOptions = {
  include?: string[];
  exclude?: string[];
};

export const expectNoWorkbenchA11yViolations = async (
  page: Page,
  options: WorkbenchA11yOptions = {},
) => {
  const { include = ['.workbench-shell'], exclude = [] } = options;
  const disabledRules = [
    'color-contrast',
    'aria-progressbar-name',
    'aria-required-children',
    'label',
    'nested-interactive',
  ];

  let builder = new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa'])
    .disableRules(disabledRules);

  include.forEach((selector) => {
    builder = builder.include(selector);
  });

  exclude.forEach((selector) => {
    builder = builder.exclude(selector);
  });

  const results = await builder.analyze();
  const blockingViolations = results.violations.filter(
    (violation) => violation.impact === 'critical' || violation.impact === 'serious',
  );

  expect(
    blockingViolations,
    `A11y blocking violations:\n${JSON.stringify(blockingViolations, null, 2)}`,
  ).toEqual([]);
};
