// Run only through run-design-checks.sh: all writes use its disposable database.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to the isolated design-check runner');
const target = new URL(base);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093',
  'Transport checks require the disposable runner on port 8093');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', error => { errors.push(error.message); console.error(error.stack); });
  try {
    await page.addInitScript(() => {
      const originalFetch = window.fetch.bind(window);
      window.transportProbe = { bodies: [], injected: 0, bodyReads: 0 };
      window.fetch = async (input, init) => {
        const request = new Request(input, init);
        const isSave = new URL(request.url).pathname.startsWith('/api/save_project_draft');
        if (isSave) window.transportProbe.bodies.push(await request.clone().text());
        const response = await originalFetch(request);
        if (!isSave || window.transportProbe.injected !== 0) return response;
        if (response.status !== 200) throw new Error(`Expected committed save, got ${response.status}`);
        // Fetch succeeds with real server headers, but reading its replacement
        // body fails. route.abort() only exercises a failure before headers.
        const bytes = new Uint8Array(await response.arrayBuffer());
        window.transportProbe.injected++;
        const interrupted = new Response(new ReadableStream({
          start(controller) { controller.enqueue(bytes.slice(0, 1)); },
          pull(controller) { controller.error(new TypeError('Interrupted response body')); },
        }), { status: response.status, headers: response.headers });
        // Constructed Responses have an empty URL; preserve it so the request
        // reaches body decoding instead of failing Dioxus's earlier URL parse.
        Object.defineProperty(interrupted, 'url', { value: response.url });
        const readBody = interrupted.arrayBuffer.bind(interrupted);
        interrupted.arrayBuffer = () => {
          window.transportProbe.bodyReads++;
          return readBody();
        };
        return interrupted;
      };
    });

    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    await page.getByRole('link', { name: 'Projects', exact: true }).click();
    await page.getByRole('link', { name: 'New project', exact: true }).click();
    const screen = page.locator('.np-page');
    const draftStatus = screen.locator('header').getByRole('status');
    await expect(draftStatus).toHaveText('No draft saved yet');

    await screen.getByLabel('Project name', { exact: true }).fill('Body interrupted after commit');
    await expect.poll(() => page.evaluate(() => window.transportProbe.injected)).toBe(1);
    await expect.poll(() => page.evaluate(() => window.transportProbe.bodyReads)).toBe(1);
    await expect(draftStatus).toHaveText('Changes need attention');
    await expect(screen.getByRole('alert')).toBeVisible();
    await screen.getByLabel('Project name', { exact: true }).fill('Latest edit survives body failure');
    await screen.getByRole('button', { name: 'Retry request', exact: true }).click();
    await expect(draftStatus).toContainText('Draft saved at');
    const bodies = await page.evaluate(() => window.transportProbe.bodies);
    assert.equal(bodies.length, 3, 'Retry acknowledges the uncertain save before sending newer edits');
    assert.equal(bodies[1], bodies[0], 'Retry preserves the committed request identity and snapshot');
    assert.notEqual(bodies[2], bodies[0], 'Newer edits are sent only after the retry');
    await screen.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await page.getByRole('link', { name: 'New project', exact: true }).click();
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Latest edit survives body failure');
    assert.deepEqual(errors, [], 'A response body failure must not panic the WASM application');
    console.log('PASS: interrupted response bodies remain retryable without losing newer edits');
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
