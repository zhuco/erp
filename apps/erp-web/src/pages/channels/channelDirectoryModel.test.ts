import { describe, expect, it } from 'vitest';

import type { Channel, ChannelCreatePayload, ChannelUpdatePayload } from '@/api/channels';
import {
  createMockChannel,
  getMockChannelList,
  updateMockChannel,
} from '@/api/mocks/masterData';

type ExtendedChannel = Channel & {
  channel_type?: 'ONLINE' | 'OFFLINE';
  contact_name?: string | null;
  contact_phone?: string | null;
  contact_address?: string | null;
};

describe('channel directory mock data', () => {
  it('stores channel type and contact fields on newly created channels', () => {
    const created = createMockChannel({
      platform: 'taobao',
      name: '淘宝直营网店',
      channel_type: 'ONLINE',
      contact_name: '李青',
      contact_phone: '13800001111',
      contact_address: '上海市浦东新区',
      allow_manual_order: true,
      allow_manual_after_sales: false,
    } as ChannelCreatePayload & {
      channel_type: 'ONLINE';
      contact_name: string;
      contact_phone: string;
      contact_address: string;
    }) as ExtendedChannel;

    expect(created.channel_type).toBe('ONLINE');
    expect(created.contact_name).toBe('李青');
    expect(created.contact_phone).toBe('13800001111');
    expect(created.contact_address).toContain('浦东');
  });

  it('supports switching a channel to offline without keeping a platform code', () => {
    const created = createMockChannel({
      platform: 'jd',
      name: '京东直营网店',
      channel_type: 'ONLINE',
      contact_name: '王敏',
      contact_phone: '13900002222',
      contact_address: '北京市朝阳区',
      allow_manual_order: true,
      allow_manual_after_sales: true,
    } as ChannelCreatePayload & {
      channel_type: 'ONLINE';
      contact_name: string;
      contact_phone: string;
      contact_address: string;
    }) as ExtendedChannel;

    const updated = updateMockChannel(
      created.channel_id,
      {
        channel_type: 'OFFLINE',
        platform: null,
        contact_name: '王敏',
        contact_phone: '13900002222',
        contact_address: '北京市海淀区',
      } as ChannelUpdatePayload & {
        channel_type: 'OFFLINE';
        platform: null;
        contact_name: string;
        contact_phone: string;
        contact_address: string;
      },
    ) as ExtendedChannel;

    expect(updated.channel_type).toBe('OFFLINE');
    expect(updated.platform).toBeNull();
    expect(updated.contact_address).toContain('海淀');
  });

  it('allows keyword search to hit contact fields in channel directory data', () => {
    const created = createMockChannel({
      platform: 'pdd',
      name: '拼多多旗舰店渠道',
      channel_type: 'ONLINE',
      contact_name: '赵云',
      contact_phone: '13700003333',
      contact_address: '杭州市滨江区',
      allow_manual_order: false,
      allow_manual_after_sales: true,
    } as ChannelCreatePayload & {
      channel_type: 'ONLINE';
      contact_name: string;
      contact_phone: string;
      contact_address: string;
    }) as ExtendedChannel;

    const byContact = getMockChannelList({ keyword: '赵云', offset: 0, limit: 200 }).items;
    const byPhone = getMockChannelList({ keyword: '13700003333', offset: 0, limit: 200 }).items;

    expect(byContact.some((item) => item.channel_id === created.channel_id)).toBe(true);
    expect(byPhone.some((item) => item.channel_id === created.channel_id)).toBe(true);
  });
});
