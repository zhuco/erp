import { describe, expect, it } from 'vitest';

import { CompensationPage } from './CompensationPage';

describe('CompensationPage', () => {
  it('renders the compensation placeholder shell', () => {
    const element = CompensationPage();

    expect(element).toBeTruthy();
    expect(element.props['data-testid']).toBe('insights-service-compensation-page');
  });
});
