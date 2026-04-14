import {
  Badge,
  Empty,
  Button,
  Card,
  Col,
  List,
  Progress,
  Row,
  Space,
  Statistic,
  Tag,
  Typography
} from 'antd';
import { useMemo } from 'react';
import { useNavigate } from 'react-router-dom';

import { appMenuItems, flattenMenuItems } from '@/app/layout/menu';
import { isFeatureEnabled } from '@/features/featureFlags';
import { hasPermission } from '@/features/rbac';
import { useSessionStore } from '@/features/sessionStore';
import { apiBaseUrl, buildVersion } from '@/utils/env';
import {
  buildDashboardQuickLinks,
  buildDashboardWorkbenchCards
} from './dashboardWorkbenchModel';

import './dashboardPage.css';

const summaryCards = [
  { key: 'review', title: '待复核订单', value: 128, suffix: '单', trend: '+12', trendColor: 'blue' },
  { key: 'ship', title: '待发货订单', value: 86, suffix: '单', trend: '+6', trendColor: 'gold' },
  { key: 'stock', title: '库存预警', value: 23, suffix: 'SKU', trend: '需关注', trendColor: 'red' },
  { key: 'after', title: '售后待处理', value: 14, suffix: '单', trend: '+2', trendColor: 'orange' }
];

const todoList = [
  { key: 'task-1', title: '异常订单待复核', count: 7, level: '紧急', color: 'red' },
  { key: 'task-2', title: '物流同步失败', count: 3, level: '处理中', color: 'gold' },
  { key: 'task-3', title: '待确认库存调整', count: 5, level: '提醒', color: 'blue' },
  { key: 'task-4', title: '售后工单待分配', count: 4, level: '提醒', color: 'purple' }
];

const alerts = [
  { key: 'alert-1', title: '华东仓库存低于安全线', time: '10:30' },
  { key: 'alert-2', title: '今日订单量高于昨日 18%', time: '09:50' },
  { key: 'alert-3', title: '打印组件版本可更新', time: '09:20' },
  { key: 'alert-4', title: '售后 SLA 即将到期', time: '08:55' }
];

const performanceItems = [
  { key: 'perf-1', label: '订单处理进度', percent: 72 },
  { key: 'perf-2', label: '出库执行进度', percent: 58 },
  { key: 'perf-3', label: '库存同步完成度', percent: 84 }
];

export const DashboardPage = () => {
  const navigate = useNavigate();
  const permissions = useSessionStore((state) => state.permissions);
  const featureFlags = useSessionStore((state) => state.featureFlags);
  const capabilityMatrix = useSessionStore((state) => state.capabilityMatrix);

  const accessibleMenuLeaves = useMemo(
    () =>
      flattenMenuItems(appMenuItems).filter((item) => {
        if (item.permission && !hasPermission(permissions, item.permission)) {
          return false;
        }
        if (item.feature && !isFeatureEnabled(featureFlags, item.feature)) {
          return false;
        }
        return true;
      }),
    [featureFlags, permissions]
  );
  const roleCards = useMemo(
    () => buildDashboardWorkbenchCards(accessibleMenuLeaves),
    [accessibleMenuLeaves]
  );
  const quickLinks = useMemo(
    () => buildDashboardQuickLinks(accessibleMenuLeaves),
    [accessibleMenuLeaves]
  );
  const enabledFeatures = Object.values(featureFlags).filter((flag) => flag === 'enabled').length;
  const capabilityCount = Object.values(capabilityMatrix).reduce(
    (count, platformCaps) => count + Object.values(platformCaps).filter(Boolean).length,
    0
  );

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <Card
        title="岗位作业台"
        extra={<Typography.Text type="secondary">按当前权限自动推荐可执行工作面</Typography.Text>}
      >
        {roleCards.length ? (
          <Row gutter={[16, 16]}>
            {roleCards.map((card) => (
              <Col xs={24} md={12} xl={6} key={card.key}>
                <div
                  className="dashboard-role-card"
                  data-testid={`dashboard-role-card-${card.key}`}
                >
                  <Space direction="vertical" size={12} style={{ width: '100%' }}>
                    <div className="dashboard-role-header">
                      <Space direction="vertical" size={4}>
                        <Typography.Title level={5} style={{ margin: 0 }}>
                          {card.title}
                        </Typography.Title>
                        <Typography.Text type="secondary">{card.summary}</Typography.Text>
                      </Space>
                      <Tag color="blue">{card.availableCount} 项可达</Tag>
                    </div>

                    <Space wrap size={[8, 8]}>
                      {card.focusTags.map((tag) => (
                        <Tag key={`${card.key}-${tag}`} className="dashboard-role-tag">
                          {tag}
                        </Tag>
                      ))}
                    </Space>

                    <Space wrap size={[8, 8]}>
                      <Button
                        type="primary"
                        data-testid={`dashboard-role-primary-${card.key}`}
                        onClick={() => navigate(card.primaryAction.path)}
                      >
                        {card.primaryAction.label}
                      </Button>
                      {card.actions.map((action) => (
                        <Button
                          key={`${card.key}-${action.path}`}
                          onClick={() => navigate(action.path)}
                        >
                          {action.label}
                        </Button>
                      ))}
                    </Space>
                  </Space>
                </div>
              </Col>
            ))}
          </Row>
        ) : (
          <Empty description="当前账号暂无可推荐的岗位工作台" />
        )}
      </Card>

      <Row gutter={[16, 16]}>
        {summaryCards.map((item) => (
          <Col xs={24} md={12} lg={6} key={item.key}>
            <Card className="dashboard-summary-card">
              <Space direction="vertical" size={8}>
                <Statistic title={item.title} value={item.value} suffix={item.suffix} />
                <Tag color={item.trendColor} className="dashboard-summary-tag">
                  {item.trend}
                </Tag>
              </Space>
            </Card>
          </Col>
        ))}
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={14}>
          <Card title="待处理任务">
            <List
              dataSource={todoList}
              renderItem={(item) => (
                <List.Item>
                  <Space style={{ width: '100%', justifyContent: 'space-between' }}>
                    <Typography.Text>{item.title}</Typography.Text>
                    <Space>
                      <Tag color={item.color}>{item.level}</Tag>
                      <Badge count={item.count} showZero />
                    </Space>
                  </Space>
                </List.Item>
              )}
            />
          </Card>
        </Col>
        <Col xs={24} lg={10}>
          <Card title="系统运行">
            <Space direction="vertical" size={12} style={{ width: '100%' }}>
              {performanceItems.map((item) => (
                <div key={item.key}>
                  <Space style={{ width: '100%', justifyContent: 'space-between' }}>
                    <Typography.Text>{item.label}</Typography.Text>
                    <Typography.Text type="secondary">{item.percent}%</Typography.Text>
                  </Space>
                  <Progress percent={item.percent} size="small" />
                </div>
              ))}
              <Space size={16}>
                <Statistic title="权限数" value={permissions.length} />
                <Statistic title="启用功能" value={enabledFeatures} />
                <Statistic title="能力项" value={capabilityCount} />
              </Space>
            </Space>
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={14}>
          <Card title="快捷入口">
            <Space wrap size={[8, 8]}>
              {quickLinks.map((link) => (
                <Button key={link.path} onClick={() => navigate(link.path)}>
                  {link.label}
                </Button>
              ))}
            </Space>
          </Card>
        </Col>
        <Col xs={24} lg={10}>
          <Card title="近期提醒">
            <List
              dataSource={alerts}
              renderItem={(item) => (
                <List.Item>
                  <Space style={{ width: '100%', justifyContent: 'space-between' }}>
                    <Typography.Text>{item.title}</Typography.Text>
                    <Typography.Text type="secondary">{item.time}</Typography.Text>
                  </Space>
                </List.Item>
              )}
            />
          </Card>
        </Col>
      </Row>

      <Row gutter={[16, 16]}>
        <Col xs={24} lg={12}>
          <Card title="环境信息">
            <Space direction="vertical" size={8}>
              <Typography.Text>API 地址：{apiBaseUrl}</Typography.Text>
              <Typography.Text>Build 版本：{buildVersion}</Typography.Text>
            </Space>
          </Card>
        </Col>
        <Col xs={24} lg={12}>
          <Card title="运营建议">
            <Space direction="vertical" size={8}>
              <Typography.Text>重点关注：高频重复异常订单。</Typography.Text>
              <Typography.Text>建议动作：完善订单审核规则与黑名单。</Typography.Text>
              <Typography.Text>本周目标：提升发货及时率到 98%。</Typography.Text>
            </Space>
          </Card>
        </Col>
      </Row>
    </Space>
  );
};
