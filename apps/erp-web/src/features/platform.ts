export enum Platform {
  Taobao = 'taobao',
  Jd = 'jd',
  Pdd = 'pdd',
  Douyin = 'douyin'
}

export type PlatformCode = (typeof Platform)[keyof typeof Platform];

export const platformLabels: Record<PlatformCode, string> = {
  [Platform.Taobao]: '淘宝',
  [Platform.Jd]: '京东',
  [Platform.Pdd]: '拼多多',
  [Platform.Douyin]: '抖音'
};

export const platformValues = Object.values(Platform) as PlatformCode[];

export const platformOptions = platformValues.map((value) => ({
  value,
  label: platformLabels[value]
}));
