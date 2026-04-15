import { Card, Empty, Typography } from 'antd';

import { UiModule02WorkbenchTemplate } from '@/components/ui-modules';

export const ProductDiffPage = () => (
  <UiModule02WorkbenchTemplate
    header={
      <Typography.Title level={3} style={{ margin: 0 }}>
        货品差异
      </Typography.Title>
    }
    tableContent={
      <Card>
        <Empty description="功能建设中" />
      </Card>
    }
  />
);
