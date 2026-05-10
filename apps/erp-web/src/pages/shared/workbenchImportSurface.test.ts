import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('workbench import surface contracts', () => {
  it('imports UiModule05DetailPanel from ui-modules instead of page-templates', () => {
    const channelPanelsSource = readFileSync(
      resolve(__dirname, '../channels/ChannelDirectoryPanels.tsx'),
      'utf8',
    );
    const couriersSource = readFileSync(resolve(__dirname, '../couriers/CouriersPage.tsx'), 'utf8');

    expect(channelPanelsSource).toContain("import { UiModule05DetailPanel } from '@/components/ui-modules';");
    expect(channelPanelsSource).not.toContain('UiModule05DetailPanel,\n} from \'@/components/page-templates\';');
    expect(couriersSource).toContain("import { UiModule04BottomTable, UiModule05DetailPanel } from '@/components/ui-modules';");
    expect(couriersSource).not.toContain('UiModule05DetailPanel,\n} from \'@/components/page-templates\';');
  });

  it('keeps the industrial compact grid preset exported from the shared component surface', () => {
    const pageTemplateExports = readFileSync(
      resolve(__dirname, '../../components/page-templates/index.ts'),
      'utf8',
    );

    expect(pageTemplateExports).toContain("export * from './workbench';");
  });
});
