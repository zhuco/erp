import {
  CheckOutlined,
  CloudUploadOutlined,
  DeleteOutlined,
  QuestionCircleOutlined,
  SettingOutlined
} from '@ant-design/icons';
import { Alert, Button, Card, Space, Table, Tabs, Tag } from 'antd';
import type { ColumnsType } from 'antd/es/table';
import type { Key } from 'react';
import { useState } from 'react';

type ImportOrderRow = {
  key: string;
  importCode: string;
  verifyResult: string;
  shopOrderNo: string;
  orderTime: string;
  payTime: string;
  customerName: string;
  customerAccount: string;
  receiverName: string;
  mobile: string;
  phone: string;
  province: string;
  city: string;
  district: string;
  street: string;
  address: string;
};

type ImportOrderItemRow = {
  key: string;
  skuCode: string;
  skuName: string;
  spec: string;
  unit: string;
  quantity: number;
  price: number;
  discount: string;
  tax: string;
  amount: number;
  terminalPrice: number;
  terminalAmount: number;
  remark: string;
};

const importRows: ImportOrderRow[] = [
  {
    key: '1',
    importCode: 'IMP-2026-0001',
    verifyResult: '校验通过',
    shopOrderNo: 'SHOP-2026-00088',
    orderTime: '2026-01-20 12:45:00',
    payTime: '2026-01-20 12:50:00',
    customerName: '广州百货',
    customerAccount: 'GZBH-001',
    receiverName: '张晓晴',
    mobile: '13800001111',
    phone: '020-88886666',
    province: '广东省',
    city: '广州市',
    district: '天河区',
    street: '科技路',
    address: '科技路88号A座'
  }
];

const importItemRows: ImportOrderItemRow[] = [
  {
    key: '1',
    skuCode: 'SKU-001',
    skuName: '示例货品A',
    spec: '标准版',
    unit: '件',
    quantity: 2,
    price: 199,
    discount: '无',
    tax: '13%',
    amount: 398,
    terminalPrice: 199,
    terminalAmount: 398,
    remark: ''
  }
];

const importColumns: ColumnsType<ImportOrderRow> = [
  { title: '导入编号', dataIndex: 'importCode', width: 140, fixed: 'left' },
  {
    title: '校验结果',
    dataIndex: 'verifyResult',
    width: 100,
    render: (value: string) => <Tag color="green">{value}</Tag>
  },
  { title: '网店订单号', dataIndex: 'shopOrderNo', width: 150 },
  { title: '下单时间', dataIndex: 'orderTime', width: 160 },
  { title: '付款时间', dataIndex: 'payTime', width: 160 },
  { title: '客户名称', dataIndex: 'customerName', width: 120 },
  { title: '客户账号', dataIndex: 'customerAccount', width: 120 },
  { title: '收货人', dataIndex: 'receiverName', width: 100 },
  { title: '手机号', dataIndex: 'mobile', width: 120 },
  { title: '固定电话', dataIndex: 'phone', width: 120 },
  { title: '省份', dataIndex: 'province', width: 100 },
  { title: '市(区)', dataIndex: 'city', width: 100 },
  { title: '区(县)', dataIndex: 'district', width: 100 },
  { title: '街道(乡镇)', dataIndex: 'street', width: 120 },
  { title: '收货地址', dataIndex: 'address', width: 200 }
];

const importItemColumns: ColumnsType<ImportOrderItemRow> = [
  { title: '货品编号', dataIndex: 'skuCode', width: 120 },
  { title: '货品名称', dataIndex: 'skuName', width: 160 },
  { title: '规格', dataIndex: 'spec', width: 100 },
  { title: '单位', dataIndex: 'unit', width: 80 },
  { title: '数量', dataIndex: 'quantity', width: 80 },
  { title: '单价', dataIndex: 'price', width: 100 },
  { title: '优惠(折扣)', dataIndex: 'discount', width: 120 },
  { title: '税额(税率)', dataIndex: 'tax', width: 100 },
  { title: '金额', dataIndex: 'amount', width: 100 },
  { title: '终端销售单价', dataIndex: 'terminalPrice', width: 120 },
  { title: '终端销售金额', dataIndex: 'terminalAmount', width: 120 },
  { title: '备注', dataIndex: 'remark', width: 120 }
];

export const SalesOrderImportPage = () => {
  const [selectedRowKeys, setSelectedRowKeys] = useState<Key[]>([]);

  return (
    <Space direction="vertical" size={16} style={{ width: '100%' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <Space>
          <Button type="primary" icon={<CloudUploadOutlined />}>
            导入
          </Button>
          <Button icon={<CheckOutlined />}>提交</Button>
          <Button danger icon={<DeleteOutlined />}>
            删除
          </Button>
        </Space>
        <Space>
          <Button icon={<SettingOutlined />}>设置</Button>
          <Button icon={<QuestionCircleOutlined />}>帮助</Button>
        </Space>
      </div>
      <Alert
        type="warning"
        showIcon
        message="导入当前页面的订单后，需要先创建订单，否则无法在订单查询页面查看。"
      />
      <Card size="small">
        <Table<ImportOrderRow>
          rowKey="key"
          columns={importColumns}
          dataSource={importRows}
          size="small"
          rowSelection={{
            selectedRowKeys,
            onChange: (keys) => setSelectedRowKeys(keys)
          }}
          pagination={{ size: 'small', pageSize: 10 }}
          scroll={{ x: 1600, y: 320 }}
        />
      </Card>
      <Card size="small">
        <Tabs
          size="small"
          items={[
            {
              key: 'items',
              label: '货品明细',
              children: (
                <Table<ImportOrderItemRow>
                  rowKey="key"
                  columns={importItemColumns}
                  dataSource={importItemRows}
                  size="small"
                  pagination={false}
                  scroll={{ x: 1200 }}
                />
              )
            }
          ]}
        />
      </Card>
    </Space>
  );
};
