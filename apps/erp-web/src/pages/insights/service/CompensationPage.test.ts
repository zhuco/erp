import { describe, expect, it } from 'vitest';

import { CompensationPage } from './CompensationPage';
import { ReshipQualityPage } from './ReshipQualityPage';
import { ReturnLoopPage } from './ReturnLoopPage';
import { TicketEfficiencyPage } from './TicketEfficiencyPage';

describe('service insight pages', () => {
  it('exports service insight placeholders with stable test ids', () => {
    expect(ReturnLoopPage().props['data-testid']).toBe('insights-service-return-loop-page');
    expect(CompensationPage().props['data-testid']).toBe('insights-service-compensation-page');
    expect(ReshipQualityPage().props['data-testid']).toBe('insights-service-reship-quality-page');
    expect(TicketEfficiencyPage().props['data-testid']).toBe('insights-service-ticket-efficiency-page');
  });
});
