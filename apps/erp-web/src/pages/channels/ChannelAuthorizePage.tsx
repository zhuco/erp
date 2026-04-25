import { useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';

import { PlaceholderPage } from '@/pages/Placeholders';

export const ChannelAuthorizePage = () => {
  const [searchParams] = useSearchParams();
  const channelId = useMemo(() => searchParams.get('channel_id') ?? '', [searchParams]);

  return (
    <PlaceholderPage
      title="渠道授权页"
      description={
        channelId
          ? `正在为渠道 ${channelId} 预留授权入口，后续会接入正式授权流程与回调处理。`
          : '正在预留渠道授权入口，后续会接入正式授权流程与回调处理。'
      }
      testId="channel-authorize-page"
      titleTestId="channel-authorize-title"
    />
  );
};
