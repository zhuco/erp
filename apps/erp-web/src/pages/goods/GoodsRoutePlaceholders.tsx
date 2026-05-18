import { Alert, Button, Card, Empty, Flex, Typography } from 'antd';
import { useNavigate } from 'react-router-dom';

type GoodsRoutePlaceholderProps = {
  title: string;
  description: string;
  backTo?: string;
  backLabel?: string;
};

const GoodsRoutePlaceholder = ({
  title,
  description,
  backTo = '/dashboard',
  backLabel = '返回工作台',
}: GoodsRoutePlaceholderProps) => {
  const navigate = useNavigate();

  return (
    <div style={{ padding: 24 }}>
      <Card bordered={false}>
        <Flex vertical gap={16} style={{ width: '100%' }}>
          <Typography.Title level={3} style={{ margin: 0 }}>
            {title}
          </Typography.Title>
          <Alert
            type="info"
            showIcon
            message="货品模块基础路由已接入"
            description={description}
          />
          <Empty description="业务页面正在接入现有接口" />
          <Button type="primary" onClick={() => navigate(backTo)}>
            {backLabel}
          </Button>
        </Flex>
      </Card>
    </div>
  );
};

export const GoodsProductsCreatePlaceholderPage = () => (
  <GoodsRoutePlaceholder
    title="货品新建"
    description="该入口已经纳入货品模块，下一步将接入可实际使用的货品新建表单。"
  />
);

export const GoodsProductsQueryPlaceholderPage = () => (
  <GoodsRoutePlaceholder
    title="货品查询"
    description="该查询入口已经纳入统一工作台路由，下一步将接入货品查询、详情与编辑能力。"
    backTo="/goods/products/new"
    backLabel="前往货品新建"
  />
);

export const GoodsKitsCreatePlaceholderPage = () => (
  <GoodsRoutePlaceholder
    title="新建组合装"
    description="该入口已经纳入货品模块，下一步将接入组合装新建与 BOM 编辑能力。"
  />
);

export const GoodsKitsQueryPlaceholderPage = () => (
  <GoodsRoutePlaceholder
    title="组合装查询"
    description="该查询入口已经纳入统一工作台路由，下一步将接入组合装查询、详情与编辑能力。"
    backTo="/goods/kits/new"
    backLabel="前往新建组合装"
  />
);
