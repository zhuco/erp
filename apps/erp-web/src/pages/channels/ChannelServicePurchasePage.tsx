import { useMemo } from 'react';
import { useSearchParams } from 'react-router-dom';

import { PlaceholderPage } from '@/pages/Placeholders';

export const ChannelServicePurchasePage = () => {
  const [searchParams] = useSearchParams();
  const channelId = useMemo(() => searchParams.get('channel_id') ?? '', [searchParams]);

  return (
    <PlaceholderPage
      title="渠道服务购买页"
      description={
        channelId
          ? `正在为渠道 ${channelId} 预留服务购买入口，后续会接入正式套餐与订购流程。`
          : '正在预留渠道服务购买入口，后续会接入正式套餐与订购流程。'
      }
      testId="channel-service-purchase-page"
      titleTestId="channel-service-purchase-title"
    />
  );
};
