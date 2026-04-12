console.log('before require');
const { chromium } = require('@playwright/test');
console.log('before launch');
(async () => {
  const browser = await chromium.launch({ headless: true });
  console.log('after launch');
  await browser.close();
})();
