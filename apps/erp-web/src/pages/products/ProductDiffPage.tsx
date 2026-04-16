import { Card, Empty, Typography } from 'antd';

import { WorkbenchMainTableTemplate } from '@/components/workbench/templates';

export const ProductDiffPage = () => (
  <WorkbenchMainTableTemplate
    header={
      <Typography.Title level={3} style={{ margin: 0 }}>
        商品差异
      </Typography.Title>
    }
    tableContent={
      <Card>
        <Empty description="功能建设中" />
      </Card>
    }
  />
);

