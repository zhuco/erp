export type TemplatePageUsage = {
  file: string;
  applicable: boolean;
  hasTemplate: boolean;
};

export type TemplateAdoptionRate = {
  applicableTotal: number;
  templateTotal: number;
  rate: number;
};

const acceptedTemplateTokens = [
  'WorkbenchPageTemplate',
  'SettingsPageTemplate',
  'UiModule03MainTableSection',
  'PlaceholderPage',
] as const;

const legacyTemplateTokens = [
  'OrderReviewWorkbenchTemplate',
  'OrderReviewWorkbenchToolbarTemplate',
  'UiModule07SettingsLayout',
  'UiModule07SettingsSection',
] as const;

export const calculateTemplateAdoptionRate = (pages: TemplatePageUsage[]): TemplateAdoptionRate => {
  const applicablePages = pages.filter((page) => page.applicable);
  const templateTotal = applicablePages.filter((page) => page.hasTemplate).length;

  return {
    applicableTotal: applicablePages.length,
    templateTotal,
    rate: applicablePages.length === 0 ? 1 : templateTotal / applicablePages.length,
  };
};

export const isTemplateAdoptionPassing = (rate: number) => rate >= 0.95;

export const classifyTemplateUsage = (source: string) => ({
  hasTemplate: acceptedTemplateTokens.some((token) => source.includes(token)),
  hasLegacyTemplate: legacyTemplateTokens.some((token) => source.includes(token)),
});
