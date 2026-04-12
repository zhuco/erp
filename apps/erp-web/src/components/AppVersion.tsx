import { Typography } from 'antd';

import { buildVersion } from '@/utils/env';

export const AppVersion = () => {
    return (
        <Typography.Text type="secondary" style={{ fontSize: '12px' }}>
            v{buildVersion}
        </Typography.Text>
    );
};
