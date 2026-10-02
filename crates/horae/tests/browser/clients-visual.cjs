// Read-only UI capture and keyboard acceptance on the disposable runner.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const base = process.env.HORAE_TEST_URL;
const target = new URL(base);
const database = new URL(process.env.DATABASE_URL);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093');
assert.equal(database.pathname, '/horae');
assert.ok(database.searchParams.get('host')?.startsWith('/tmp/horae-browser.'));
const output = process.env.HORAE_TEST_SCREENSHOT_DIR;
if (output) fs.mkdirSync(output, { recursive: true });

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage();
  const errors = [], findings = [], captures = [];
  page.on('pageerror', error => errors.push(error.message));
  async function capture(label, scope = 'main') {
    await page.evaluate(() => document.fonts.ready);
    await page.evaluate(() => window.scrollTo(0, 0));
    if (label.endsWith('-form')) await page.locator('dialog[open] > .modal').evaluate(panel => { panel.scrollTop = 0; });
    const geometry = await page.locator(scope).evaluate(root => {
      const problems = [];
      if (document.documentElement.scrollWidth > innerWidth + 1) problems.push('Page horizontal overflow');
      for (const el of root.querySelectorAll('button, input, select, textarea, .page-title')) {
        if (!el.getClientRects().length || el.closest('dialog:not([open]), [popover]:not(:popover-open)')) continue;
        const box = el.getBoundingClientRect();
        // Tables intentionally scroll inside their own region.
        if (!el.closest('table') && (box.left < -1 || box.right > innerWidth + 1)) problems.push(`Outside viewport: ${el.id || el.textContent}`);
        if (el.tagName === 'BUTTON' && el.scrollWidth > el.clientWidth + 1) problems.push(`Clipped button: ${el.textContent}`);
      }
      const heading = root.querySelector('h1, h2');
      const rem = parseFloat(getComputedStyle(document.documentElement).fontSize);
      for (const cell of root.querySelectorAll('table tbody tr > td:first-child')) {
        if (cell.getBoundingClientRect().width < 16.25 * rem - 1) problems.push('Identity column is narrower than the handoff minimum');
      }
      const search = root.querySelector('input[type="search"]');
      if (search && innerWidth >= 1440 && rem === 16) {
        const action = root.querySelector('.page-actions button');
        if (Math.abs(search.getBoundingClientRect().top - action.getBoundingClientRect().top) > 2) problems.push('Desktop search wraps below actions');
      }
      const grid = root.querySelector('.client-detail-grid');
      if (grid) {
        const [projects, billing] = [...grid.children].map(element => element.getBoundingClientRect());
        if (Math.abs(projects.top - billing.top) < 1 && projects.width < billing.width) problems.push('Billing crowds out the primary project panel');
      }
      const luminance = color => {
        const rgb = color.match(/[\d.]+/g).slice(0, 3).map(Number).map(value => {
          const v = value / 255;
          return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
        });
        return rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
      };
      const rateHint = root.querySelector('#client-form-rate')?.closest('.form-group').querySelector('p');
      for (const element of [...root.querySelectorAll('th'), search, rateHint].filter(Boolean)) {
        let background = element;
        while (background.parentElement && ['rgba(0, 0, 0, 0)', 'transparent'].includes(getComputedStyle(background).backgroundColor)) background = background.parentElement;
        const bg = luminance(getComputedStyle(background).backgroundColor);
        const fg = luminance(getComputedStyle(element, element === search ? '::placeholder' : null).color);
        const ratio = (Math.max(fg, bg) + 0.05) / (Math.min(fg, bg) + 0.05);
        if (ratio < 4.5) problems.push(`Text contrast ${ratio.toFixed(2)}: ${element.textContent || element.placeholder}`);
      }
      return { problems, theme: document.documentElement.dataset.theme, rootFont: getComputedStyle(document.documentElement).fontSize,
        headingFont: heading ? getComputedStyle(heading).fontFamily : null,
        fonts: [...document.fonts].filter(font => font.status === 'loaded').map(font => font.family) };
    });
    findings.push(...geometry.problems.map(problem => `${label}: ${problem}`));
    if (output) await page.screenshot({ path: path.join(output, `${label}.png`), fullPage: true, animations: 'disabled' });
    captures.push({ label, ...geometry });
  }
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin', exact: true }).click();
    await page.waitForURL(`${base}/`);
    const cases = [320, 390, 768, 1440].flatMap(width => [1, 2].map(scale => ({ width, height: 900, scale })))
      .concat({ width: 1440, height: 360, scale: 1 });
    for (const theme of ['dark', 'light']) {
      await page.evaluate(theme => window.setHoraeTheme(theme), theme);
      for (const { width, height, scale } of cases) {
        const label = `${theme}-${width}x${height}-${scale * 100}`;
        await page.setViewportSize({ width, height });
        await page.goto(`${base}/clients`);
        const acme = page.getByRole('link', { name: 'Acme Corp', exact: true });
        await expect(acme).toBeVisible();
        await page.evaluate(scale => { document.documentElement.style.fontSize = `${16 * scale}px`; }, scale);
        await capture(`${label}-list`);
        await acme.click();
        await expect(page.getByRole('heading', { name: 'Acme Corp', exact: true })).toBeVisible();
        await expect(page.getByRole('link', { name: /^\[ACME-01\] / })).toBeVisible();
        await expect(page.getByRole('status')).toHaveCount(0);
        await capture(`${label}-detail`);
        const edit = page.getByRole('button', { name: 'Edit client', exact: true });
        await edit.focus();
        await page.keyboard.press('Enter');
        const dialog = page.getByRole('dialog');
        const name = dialog.getByLabel('Client name', { exact: true });
        await expect(name).toBeFocused();
        assert.equal(await dialog.evaluate(element => element.matches(':modal')), true);
        await edit.evaluate(element => element.focus());
        await expect(name).toBeFocused();
        await page.keyboard.press('Shift+Tab');
        // Permit the observed browser-level focus stop, never a background control.
        if (await page.evaluate(() => document.activeElement === document.body)) await page.keyboard.press('Shift+Tab');
        await expect(dialog.getByRole('button', { name: 'Cancel', exact: true })).toBeFocused();
        await page.keyboard.press('Tab');
        if (await page.evaluate(() => document.activeElement === document.body)) await page.keyboard.press('Tab');
        await expect(name).toBeFocused();
        await capture(`${label}-form`, 'dialog[open]');
        if (scale === 2 || height === 360) {
          await dialog.getByRole('button', { name: 'Cancel', exact: true }).focus();
          await capture(`${label}-form-actions`, 'dialog[open]');
        }
        await page.keyboard.press('Escape');
        await expect(dialog).toBeHidden();
        await expect(edit).toBeFocused();
        console.log(`PASS: clients ${label} keyboard open/trap/Escape/focus return; captured list/detail/form`);
      }
    }
    if (output) fs.writeFileSync(path.join(output, 'manifest.json'), JSON.stringify({ captures, findings, errors }, null, 2));
    assert.deepEqual(errors, []);
    assert.deepEqual(findings, [], 'Capture matrix layout findings');
  } finally { await browser.close(); }
})().catch(error => { console.error(error); process.exitCode = 1; });
