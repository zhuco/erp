import { CloudUploadOutlined } from '@ant-design/icons';
import { Button, Card, Col, DatePicker, Form, Input, InputNumber, Row, Select, Space, Steps, Table } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import { useNavigate } from 'react-router-dom';

import './salesOrderCreatePage.css';

type SalesOrderItem = {
  key: string;
  skuCode: string;
  skuName: string;
  spec: string;
  quantity: number;
  price: number;
  unit: string;
  discount: string;
  promotion: string;
  amount: number;
  isGift: boolean;
  remark: string;
  availableStock: number;
};

const productRows: SalesOrderItem[] = [
  {
    key: '1',
    skuCode: 'SKU-001',
    skuName: '示例货品A',
    spec: '标准版',
    quantity: 1,
    price: 199,
    unit: '件',
    discount: '无',
    promotion: '无',
    amount: 199,
    isGift: false,
    remark: '',
    availableStock: 120
  }
];

const productColumns: ColumnsType<SalesOrderItem> = [
  { title: '货品编码', dataIndex: 'skuCode', width: 120 },
  { title: '货品名称', dataIndex: 'skuName', width: 160 },
  { title: '规格', dataIndex: 'spec', width: 100 },
  { title: '数量', dataIndex: 'quantity', width: 80 },
  { title: '单价', dataIndex: 'price', width: 100 },
  { title: '单位', dataIndex: 'unit', width: 80 },
  { title: '优惠', dataIndex: 'promotion', width: 100 },
  { title: '折扣', dataIndex: 'discount', width: 80 },
  { title: '金额', dataIndex: 'amount', width: 100 },
  {
    title: '赠品',
    dataIndex: 'isGift',
    width: 80,
    render: (value: boolean) => (value ? '是' : '否')
  },
  { title: '备注', dataIndex: 'remark', width: 120 },
  { title: '可用库存', dataIndex: 'availableStock', width: 100 }
];

const salesChannelOptions = [
  { value: 'online', label: '线上渠道' },
  { value: 'offline', label: '线下渠道' },
  { value: 'distribution', label: '分销渠道' }
];

const orderTypeOptions = [
  { value: 'retail', label: '零售业务' },
  { value: 'wholesale', label: '批发业务' },
  { value: 'gift', label: '赠品业务' }
];

const employeeOptions = [
  { value: '王久立', label: '王久立' },
  { value: '陈晓明', label: '陈晓明' },
  { value: '李文婷', label: '李文婷' }
];

const logisticsOptions = [
  { value: '顺丰', label: '顺丰快递' },
  { value: '中通', label: '中通快递' },
  { value: '圆通', label: '圆通快递' }
];

const settlementOptions = [
  { value: 'cash', label: '现金收款' },
  { value: 'transfer', label: '银行转账' },
  { value: 'online', label: '线上支付' }
];

const currencyOptions = [
  { value: 'CNY', label: '人民币' },
  { value: 'USD', label: '美元' },
  { value: 'EUR', label: '欧元' }
];

const accountOptions = [
  { value: 'main', label: '主账户' },
  { value: 'wechat', label: '微信收款账户' },
  { value: 'alipay', label: '支付宝收款账户' }
];

export const SalesOrderCreatePage = () => {
  const navigate = useNavigate();
  const totalQuantity = productRows.reduce((sum, row) => sum + row.quantity, 0);
  const totalAmount = productRows.reduce((sum, row) => sum + row.amount, 0);

  return (
    <Space direction="vertical" size={12} style={{ width: '100%' }} className="sales-order-create-page">
      <div style={{ display: 'flex', justifyContent: 'flex-end', alignItems: 'center' }}>
        <Button
          size="small"
          icon={<CloudUploadOutlined />}
          onClick={() => navigate('/sales/import-order')}
        >
          导入订单
        </Button>
      </div>
      <Card>
        <Steps
          size="small"
          current={0}
          items={[
            { title: '新建订单' },
            { title: '订单审核' },
            { title: '订单通知发货' },
            { title: '完成发货' }
          ]}
        />
      </Card>
      <Form layout="vertical" size="small">
        <Space direction="vertical" size={12} style={{ width: '100%' }}>
          <Card title="订单信息" size="small">
            <Row gutter={12}>
              <Col xs={24} md={6}>
                <Form.Item label="销售渠道" name="salesChannel">
                  <Select size="small" options={salesChannelOptions} placeholder="选择销售渠道" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="业务员" name="salesPerson">
                  <Select size="small" options={employeeOptions} placeholder="选择业务员" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="订单类型" name="orderType">
                  <Select size="small" options={orderTypeOptions} placeholder="选择订单类型" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="下单时间" name="orderTime">
                  <DatePicker size="small" showTime style={{ width: '100%' }} />
                </Form.Item>
              </Col>
            </Row>
            <Row gutter={12}>
              <Col xs={24} md={6}>
                <Form.Item label="客户名称" name="customerName">
                  <Input size="small" placeholder="请输入客户名称" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="客户账号" name="customerAccount">
                  <Input size="small" placeholder="请输入客户账号" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="寄件人" name="senderName">
                  <Input size="small" placeholder="请输入寄件人" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="收货人" name="receiverName">
                  <Input size="small" placeholder="请输入收货人" />
                </Form.Item>
              </Col>
            </Row>
            <Row gutter={12}>
              <Col xs={24} md={12}>
                <Form.Item label="收货地址" name="receiverAddress">
                  <Input.Search
                    size="small"
                    placeholder="将收货人信息粘贴在此处，自动识别地址"
                    enterButton="地址解析"
                  />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="手机" name="receiverPhone">
                  <Input size="small" placeholder="请输入手机号" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="执行价格" name="pricePolicy">
                  <InputNumber size="small" style={{ width: '100%' }} min={0} placeholder="请输入" />
                </Form.Item>
              </Col>
            </Row>
            <Row gutter={12}>
              <Col xs={24} md={6}>
                <Form.Item label="网店订单号" name="shopOrderNo">
                  <Input size="small" placeholder="请输入网店订单号" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="物流公司" name="logisticsCompany">
                  <Select size="small" options={logisticsOptions} placeholder="选择物流公司" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="物流单号" name="logisticsNo">
                  <Input size="small" placeholder="请输入物流单号" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="执行价格类型" name="priceType">
                  <Select
                    size="small"
                    options={[
                      { value: 'standard', label: '标准价' },
                      { value: 'contract', label: '合同价' },
                      { value: 'activity', label: '活动价' }
                    ]}
                    placeholder="选择执行价格类型"
                  />
                </Form.Item>
              </Col>
            </Row>
            <Row gutter={12}>
              <Col xs={24} md={12}>
                <Form.Item label="客户备注" name="customerRemark">
                  <Input.TextArea rows={2} placeholder="客户备注信息" />
                </Form.Item>
              </Col>
              <Col xs={24} md={12}>
                <Form.Item label="货品信息" name="goodsRemark">
                  <Input.TextArea rows={2} placeholder="货品信息或配送要求" />
                </Form.Item>
              </Col>
            </Row>
          </Card>
          <Card title="商品明细" size="small">
            <Table<SalesOrderItem>
              columns={productColumns}
              dataSource={productRows}
              pagination={false}
              size="small"
              scroll={{ x: 1200 }}
              summary={() => (
                <Table.Summary.Row>
                  <Table.Summary.Cell index={0} colSpan={3}>
                    合计
                  </Table.Summary.Cell>
                  <Table.Summary.Cell index={3}>{totalQuantity}</Table.Summary.Cell>
                  <Table.Summary.Cell index={4} colSpan={4} />
                  <Table.Summary.Cell index={8}>{totalAmount.toFixed(2)}</Table.Summary.Cell>
                  <Table.Summary.Cell index={9} colSpan={3} />
                </Table.Summary.Row>
              )}
            />
          </Card>
          <Card title="结算信息" size="small">
            <Row gutter={12}>
              <Col xs={24} md={6}>
                <Form.Item label="结算方式" name="settlementMethod">
                  <Select size="small" options={settlementOptions} placeholder="选择结算方式" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="币种" name="currency">
                  <Select size="small" options={currencyOptions} placeholder="选择币种" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="收款账户" name="collectionAccount">
                  <Select size="small" options={accountOptions} placeholder="选择收款账户" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="支付单号" name="paymentNo">
                  <Input size="small" placeholder="请输入支付单号" />
                </Form.Item>
              </Col>
            </Row>
            <Row gutter={12}>
              <Col xs={24} md={6}>
                <Form.Item label="货款合计" name="totalAmount">
                  <InputNumber size="small" style={{ width: '100%' }} min={0} placeholder="0.00" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="返利金额" name="rebateAmount">
                  <InputNumber size="small" style={{ width: '100%' }} min={0} placeholder="0.00" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="应收合计" name="receivableAmount">
                  <InputNumber size="small" style={{ width: '100%' }} min={0} placeholder="0.00" />
                </Form.Item>
              </Col>
              <Col xs={24} md={6}>
                <Form.Item label="整单优惠" name="orderDiscount">
                  <InputNumber size="small" style={{ width: '100%' }} min={0} placeholder="0.00" />
                </Form.Item>
              </Col>
            </Row>
            <Row gutter={12}>
              <Col xs={24} md={12}>
                <Form.Item label="客服备注" name="serviceRemark">
                  <Input.TextArea rows={2} placeholder="客服备注信息" />
                </Form.Item>
              </Col>
            </Row>
          </Card>
          <div style={{ display: 'flex', justifyContent: 'center', paddingBottom: 12 }}>
            <Space>
              <Button type="primary" size="small">
                保存
              </Button>
              <Button size="small">取消</Button>
            </Space>
          </div>
        </Space>
      </Form>
    </Space>
  );
};
