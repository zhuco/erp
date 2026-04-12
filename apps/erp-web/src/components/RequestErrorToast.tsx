import { message } from 'antd';
import { useEffect } from 'react';

import { setRequestErrorReporter } from '@/utils/requestError';

export const RequestErrorToast = () => {
  const [messageApi, contextHolder] = message.useMessage();

  useEffect(() => {
    setRequestErrorReporter((content: string) => {
      messageApi.error({ content, duration: 3 });
    });
  }, [messageApi]);

  return <>{contextHolder}</>;
};
