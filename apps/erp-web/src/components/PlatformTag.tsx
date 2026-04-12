import { Tag } from 'antd';

import type { PlatformCode } from '@/features/platform';
import { platformLabels } from '@/features/platform';

const platformColors: Record<PlatformCode, string> = {
  taobao: 'magenta',
  jd: 'red',
  pdd: 'volcano',
  douyin: 'purple'
};

export const PlatformTag = ({ platform }: { platform: PlatformCode }) => (
  <Tag color={platformColors[platform]}>{platformLabels[platform]}</Tag>
);
