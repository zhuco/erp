export const ERP_TYPOGRAPHY = {
  body: 12.8,
  bodyStrong: 12.8,
  title: 15,
  titleDisplay: 18,
  denseBody: 12,
  denseBodyStrong: 12,
  denseTitle: 14,
  denseTitleDisplay: 16,
} as const;

export const resolveTypographyScale = (denseMode: boolean) => ({
  body: denseMode ? ERP_TYPOGRAPHY.denseBody : ERP_TYPOGRAPHY.body,
  bodyStrong: denseMode ? ERP_TYPOGRAPHY.denseBodyStrong : ERP_TYPOGRAPHY.bodyStrong,
  title: denseMode ? ERP_TYPOGRAPHY.denseTitle : ERP_TYPOGRAPHY.title,
  titleDisplay: denseMode ? ERP_TYPOGRAPHY.denseTitleDisplay : ERP_TYPOGRAPHY.titleDisplay,
});
