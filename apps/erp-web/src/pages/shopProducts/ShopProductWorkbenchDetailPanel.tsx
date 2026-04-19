import { Button, Descriptions, Empty, Tag, Typography } from 'antd';
import dayjs from 'dayjs';
import { useMemo } from 'react';

import type { ShopProductSku } from '@/api/shopProducts';
import { UiModule05DetailPanel } from '@/components/ui-modules/UiModule05DetailPanel';
import { platformLabels } from '@/features/platform';

type ShopProductWorkbenchDetailPanelProps = {
  activeSku: ShopProductSku | null;
  shopName?: string;
};

const toDate = (value?: string | null) => (value ? dayjs(value).format('YYYY-MM-DD HH:mm') : '-');
const toMoney = (value?: number | null) => (value == null ? '-' : (value / 100).toFixed(2));

const buildMarkTags = (activeSku: ShopProductSku) => {
  const tags = [
    activeSku.mapping_status,
    activeSku.match_source,
    activeSku.conflict_reason,
    activeSku.mapped_sku_code,
  ]
    .map((item) => String(item ?? '').trim())
    .filter(Boolean);

  return Array.from(new Set(tags));
};

const renderMatchSource = (value?: string | null) => {
  if (value === 'AUTO') {
    return <Tag color="green">AUTO</Tag>;
  }
  if (value === 'MANUAL') {
    return <Tag color="blue">MANUAL</Tag>;
  }
  return <Tag>UNMATCHED</Tag>;
};

const resolveStatusHint = (activeSku: ShopProductSku) => {
  if (activeSku.mapping_status === 'CONFLICT') {
    return '当前记录存在冲突，建议先处理失败原因，再继续自动匹配或手动匹配。';
  }
  if (activeSku.mapping_id) {
    return '当前记录已建立匹配，可继续创建 ERP 档案或复核映射结果。';
  }
  return '当前记录尚未匹配，可执行自动匹配或手动匹配。';
};

export const ShopProductWorkbenchDetailPanel = ({
  activeSku,
  shopName,
}: ShopProductWorkbenchDetailPanelProps) => {
  const markTags = useMemo(() => (activeSku ? buildMarkTags(activeSku) : []), [activeSku]);

  if (!activeSku) {
    return (
      <div
        className="shop-products-detail-workbench ui-module-05-order-review-right-panel"
        data-testid="shop-products-detail-panel"
      >
        <Empty description="请选择一条平台 SKU 查看工作台详情" />
      </div>
    );
  }

  return (
    <div
      className="shop-products-detail-workbench ui-module-05-order-review-right-panel"
      data-testid="shop-products-detail-panel"
    >
      <div data-testid="shop-products-detail-template">
        <UiModule05DetailPanel
          panels={[
            {
              key: 'marks',
              label: '商品标签',
              children: (
                <div className="ui-module-05-order-mark-inline">
                  <div className="ui-module-05-order-mark-inline-tags">
                    {markTags.map((tag) => (
                      <Tag key={tag}>{tag}</Tag>
                    ))}
                  </div>
                  <Button size="small" disabled>
                    添加
                  </Button>
                </div>
              ),
            },
            {
              key: 'detail',
              label: '商品详情',
              children: (
                <>
                  <Descriptions
                    size="small"
                    column={1}
                    bordered
                    className="ui-module-05-order-review-descriptions"
                  >
                    <Descriptions.Item label="平台 SKU ID">{activeSku.platform_sku_id}</Descriptions.Item>
                    <Descriptions.Item label="平台商品 ID">{activeSku.platform_product_id}</Descriptions.Item>
                    <Descriptions.Item label="商家 SKU">{activeSku.merchant_sku_code ?? '-'}</Descriptions.Item>
                    <Descriptions.Item label="平台">
                      {platformLabels[activeSku.platform] ?? activeSku.platform}
                    </Descriptions.Item>
                    <Descriptions.Item label="店铺">{shopName ?? activeSku.shop_id}</Descriptions.Item>
                    <Descriptions.Item label="商品标题">{activeSku.product_title}</Descriptions.Item>
                    <Descriptions.Item label="规格">
                      {activeSku.spec_text ?? activeSku.sku_title ?? '-'}
                    </Descriptions.Item>
                    <Descriptions.Item label="匹配结果">{renderMatchSource(activeSku.match_source)}</Descriptions.Item>
                    <Descriptions.Item label="映射状态">{activeSku.mapping_status ?? '-'}</Descriptions.Item>
                    <Descriptions.Item label="冲突原因">{activeSku.conflict_reason ?? '-'}</Descriptions.Item>
                    <Descriptions.Item label="ERP SKU">{activeSku.mapped_sku_code ?? '-'}</Descriptions.Item>
                    <Descriptions.Item label="ERP SKU 名称">{activeSku.mapped_sku_name ?? '-'}</Descriptions.Item>
                    <Descriptions.Item label="价格">{toMoney(activeSku.price_cent)}</Descriptions.Item>
                    <Descriptions.Item label="库存">{activeSku.stock_qty ?? '-'}</Descriptions.Item>
                    <Descriptions.Item label="平台更新时间">
                      {toDate(activeSku.platform_updated_at ?? activeSku.updated_at)}
                    </Descriptions.Item>
                  </Descriptions>

                  <Typography.Paragraph
                    className="shop-products-detail-status-hint"
                    data-testid="shop-products-detail-status-hint"
                    style={{ marginBottom: 0, marginTop: 8 }}
                    type="secondary"
                  >
                    {resolveStatusHint(activeSku)}
                  </Typography.Paragraph>
                </>
              ),
            },
          ]}
        />
      </div>
    </div>
  );
};
