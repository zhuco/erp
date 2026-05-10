import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { describe, expect, it } from 'vitest';

describe('ChannelDirectoryPage source contract', () => {
  it('implements the real channel directory workbench instead of forwarding to the legacy shops page', () => {
    const source = readFileSync(resolve(__dirname, './ChannelDirectoryPage.tsx'), 'utf8');

    expect(source).toContain("import './channelDirectoryPage.css';");
    expect(source).toContain("import { ChannelDirectoryLeftPanel, ChannelDirectoryRightPanel } from './ChannelDirectoryPanels';");
    expect(source).toContain("import { buildChannelDirectoryColumns");
    expect(source).toContain('resolveRowKey: resolveChannelRowKey');
    expect(source).toContain('<WorkbenchPageTemplate');
    expect(source).toContain('leftPanel={leftPanel}');
    expect(source).toContain('rightPanel={rightPanel}');
    expect(source).toContain('bottomPanel={bottomPanel}');
    expect(source).toContain('leftPanelVisible={true}');
    expect(source).toContain('rightPanelVisible={true}');
    expect(source).toContain('bottomPanelVisible={true}');
    expect(source).toContain('showFooterScrollbarRow={true}');
    expect(source).not.toContain('buildChannelDirectoryHeaderFilters');
    expect(source).not.toContain('headerFilters={headerFilters}');
    expect(source).not.toContain('return <ChannelsPage />');
  });

  it('keeps ChannelsPage only as a compatibility wrapper to the real directory page', () => {
    const source = readFileSync(resolve(__dirname, './ChannelsPage.tsx'), 'utf8');

    expect(source).toContain("import { ChannelDirectoryPage } from './ChannelDirectoryPage';");
    expect(source).toContain('return <ChannelDirectoryPage />;');
  });

  it('creates dedicated channel directory workbench support files in the channels page domain', () => {
    expect(existsSync(resolve(__dirname, './channelDirectoryModel.ts'))).toBe(true);
    expect(existsSync(resolve(__dirname, './channelDirectoryColumns.tsx'))).toBe(true);
    expect(existsSync(resolve(__dirname, './ChannelDirectoryPanels.tsx'))).toBe(true);
    expect(existsSync(resolve(__dirname, './channelDirectoryPage.css'))).toBe(true);
    expect(existsSync(resolve(__dirname, './ChannelServicePurchasePage.tsx'))).toBe(true);
    expect(existsSync(resolve(__dirname, './ChannelAuthorizePage.tsx'))).toBe(true);
  });

  it('keeps the channel directory main grid on single-line industrial cells instead of stacked primary cells', () => {
    const source = readFileSync(resolve(__dirname, './channelDirectoryColumns.tsx'), 'utf8');

    expect(source).toContain("field: 'channel_id'");
    expect(source).not.toContain('channel-directory-cell-primary');
  });
});
