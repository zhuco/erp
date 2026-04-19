import { Descriptions } from 'antd';

import type { Product } from '@/api/products';
import {
  getApparelPolicySummary,
  getDigitalPolicySummary,
  getFoodPolicySummary,
  getIndustryProfileLabel,
  getProductIndustryProfile,
} from '@/pages/products/productIndustry';

type ProductIndustrySummaryProps = {
  product: Product;
};

export const ProductIndustrySummary = ({ product }: ProductIndustrySummaryProps) => {
  const industryProfile = getProductIndustryProfile(product);

  if (industryProfile === 'GENERAL') {
    return (
      <Descriptions size="small" column={1} data-testid="products-industry-summary">
        <Descriptions.Item label="行业模板">
          {getIndustryProfileLabel(industryProfile)}
        </Descriptions.Item>
      </Descriptions>
    );
  }

  return (
    <Descriptions size="small" column={1} data-testid="products-industry-summary">
      <Descriptions.Item label="行业模板">{getIndustryProfileLabel(industryProfile)}</Descriptions.Item>
      {industryProfile === 'FOOD' ? (
        <Descriptions.Item label="食品效期策略">{getFoodPolicySummary(product)}</Descriptions.Item>
      ) : null}
      {industryProfile === 'APPAREL' ? (
        <Descriptions.Item label="鞋服规格策略">{getApparelPolicySummary(product)}</Descriptions.Item>
      ) : null}
      {industryProfile === 'DIGITAL' ? (
        <Descriptions.Item label="序列号策略">{getDigitalPolicySummary(product)}</Descriptions.Item>
      ) : null}
    </Descriptions>
  );
};
