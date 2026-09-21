// Mutates only the runner's disposable, seeded database on its reserved port.
const { chromium, expect } = require(process.env.PLAYWRIGHT_MODULE || 'playwright/test');
const assert = require('node:assert/strict');
const base = process.env.HORAE_TEST_URL;
assert.ok(base, 'Set HORAE_TEST_URL to the isolated design-check runner');
const target = new URL(base);
assert.ok(['localhost', '127.0.0.1'].includes(target.hostname) && target.port === '8093',
  'New-project checks require the disposable runner on port 8093');

(async () => {
  const browser = await chromium.launch({ executablePath: process.env.CHROMIUM_PATH || undefined, headless: true });
  const context = await browser.newContext({ viewport: { width: 1440, height: 900 } });
  const errors = [];
  const requests = new Map();
  const resources = new Map();
  context.on('page', tab => {
    const pending = new Set();
    const pendingResources = new Set();
    requests.set(tab, pending);
    resources.set(tab, pendingResources);
    tab.on('request', request => {
      pendingResources.add(request);
      if (new URL(request.url()).pathname.startsWith('/api/')) pending.add(request);
    });
    tab.on('requestfinished', request => { pending.delete(request); pendingResources.delete(request); });
    tab.on('requestfailed', request => { pending.delete(request); pendingResources.delete(request); });
    tab.on('pageerror', error => errors.push(error.stack || error.message));
    tab.on('console', message => {
      if (message.type() === 'error' && !message.text().includes('Failed to load resource') && !message.text().startsWith('WebSocket connection')) console.error(message.text());
    });
  });
  const page = await context.newPage();
  let draftWrites = 0;
  page.on('request', request => {
    if (request.url().includes('/api/save_project_draft')) draftWrites++;
  });
  // Destroying a document midway through a response body triggers an upstream
  // Dioxus decoder panic. Await reads when tearing down test pages; deliberately
  // interrupted mutations below still exercise the application's retry paths.
  const readsFinished = tab => expect.poll(() => requests.get(tab).size).toBe(0);
  const screen = page.locator('.np-page');
  const draftStatus = screen.locator('header').getByRole('status');
  const saved = () => expect(draftStatus).toContainText('Draft saved at');
  const chooseField = async (id, label, option) => {
    const trigger = screen.locator(`#${id}`);
    assert.equal(await trigger.evaluate(node => node.tagName), 'BUTTON', `${label} uses the shared selector`);
    await trigger.click();
    const picker = page.getByRole('dialog', { name: `Choose ${label}`, exact: true });
    await picker.getByRole('option', { name: option, exact: true }).focus();
    await page.keyboard.press('Enter');
    await expect(picker).not.toBeVisible();
    await expect(trigger).toBeFocused();
    await expect(trigger).toContainText(option);
  };
  const chooseDate = async (label, day) => {
    await screen.getByLabel(label, { exact: true }).click();
    const calendar = page.getByRole('dialog', { name: `Choose ${label}`, exact: true });
    await expect(calendar).toBeVisible();
    const wanted = new Date(day);
    for (let step = 0; step < 240; step++) {
      const visibleMonth = new Date(`1 ${await calendar.locator('.font-display').textContent()}`);
      if (visibleMonth.getMonth() === wanted.getMonth() && visibleMonth.getFullYear() === wanted.getFullYear()) break;
      await calendar.getByRole('button', { name: visibleMonth < wanted ? 'Next month' : 'Previous month' }).click();
    }
    await calendar.getByRole('button', { name: day, exact: true }).click();
    await expect(calendar).not.toBeVisible();
    await expect(screen.getByLabel(label, { exact: true })).toBeFocused();
  };
  try {
    await page.goto(`${base}/auth/login`);
    await page.getByRole('button', { name: 'Sign in as Admin' }).click();
    await page.waitForURL(`${base}/`);
    await page.getByRole('link', { name: 'Projects', exact: true }).click();
    for (const endpoint of ['project_creation_options', 'load_project_draft']) {
      const pattern = `**/api/${endpoint}*`;
      let release;
      const pending = new Promise(resolve => { release = resolve; });
      await page.route(pattern, async route => { await pending; await route.abort(); });
      try {
        if (endpoint === 'project_creation_options') {
          await page.getByRole('link', { name: 'New project', exact: true }).click();
        } else {
          await page.getByRole('button', { name: 'Retry', exact: true }).click();
        }
        await expect(page.getByRole('status').filter({ hasText: 'Loading project settings' })).toBeVisible();
        await expect(screen).toHaveCount(0);
        assert.equal(draftWrites, 0, 'Loading cannot create or replace a draft');
        release();
        await expect(page.getByRole('alert')).toContainText('Could not load project settings');
        await expect(page.getByRole('button', { name: 'Retry', exact: true })).toBeEnabled();
        await expect(page.getByRole('link', { name: 'Back to Projects', exact: true })).toBeVisible();
        await expect(screen).toHaveCount(0);
        assert.equal(draftWrites, 0, 'Failed initialization cannot create or replace a draft');
      } finally {
        release();
        await page.unroute(pattern);
      }
    }
    await page.getByRole('button', { name: 'Retry', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/new`);
    await expect(draftStatus).toHaveText('No draft saved yet');
    assert.equal(draftWrites, 0);
    console.log('PASS: initial catalog/draft failures show loading and recovery without rendering or saving an empty replacement');
    await expect(screen.getByRole('button', { name: 'Save project', exact: true })).toBeDisabled();
    const taskHint = screen.getByRole('region', { name: 'Tasks', exact: true }).locator('p').filter({ hasText: /^Everyone on the project can track/ });
    await expect(taskHint).toHaveCSS('padding-top', '10px');
    await expect(taskHint).toHaveCSS('padding-bottom', '10px');
    await expect(taskHint).toHaveCSS('gap', '8px');
    await expect(taskHint.locator('svg')).toHaveCSS('width', '14px');
    await expect(taskHint.locator('svg')).toHaveCSS('height', '14px');
    for (const [theme, wash] of [['light', 'rgba(31, 92, 77, 0.06)'], ['dark', 'rgba(79, 183, 154, 0.06)']]) {
      await page.locator('html').evaluate((root, theme) => { root.dataset.theme = theme; }, theme);
      await expect(taskHint).toHaveCSS('background-color', wash);
    }
    const heading = screen.getByRole('heading', { name: 'New project', exact: true });
    await expect(heading).toHaveCSS('font-size', '34px');
    await expect(heading).toHaveCSS('font-weight', '600');
    await expect(heading).toHaveCSS('letter-spacing', '-0.51px');
    assert.match(await heading.evaluate(node => getComputedStyle(node).fontFamily), /Newsreader/);
    await expect(screen.locator('header .uppercase')).toHaveCSS('letter-spacing', '1.44px');
    for (const title of await screen.locator('section > div > h2').all()) {
      await expect(title).toHaveCSS('font-size', '20px');
      await expect(title).toHaveCSS('margin-top', '0px');
      await expect(title).toHaveCSS('margin-bottom', '0px');
    }
    const back = screen.getByRole('button', { name: 'Back to Projects', exact: true });
    await expect(back).toHaveCSS('padding', '6px 10px');
    await expect(back).toHaveCSS('margin-left', '-10px');
    await expect(back).toHaveCSS('color', 'rgb(162, 156, 141)');
    for (const label of await screen.locator('.np-label > label, .np-label > div').all()) {
      await expect(label).toHaveCSS('font-size', '14px');
      await expect(label).toHaveCSS('font-weight', '600');
      assert.match(await label.evaluate(node => getComputedStyle(node).fontFamily), /Instrument Sans/);
    }
    for (const divider of await screen.locator('.np-row, header, #np-invoice-heading').all()) {
      const border = await divider.evaluate(node => getComputedStyle(node.matches('h2') ? node.parentElement : node).borderBottomColor);
      assert.equal(border, 'rgb(38, 34, 25)');
    }
    console.log('PASS: design typography, section spacing, quiet dividers and theme-aware task information');
    if (process.env.HORAE_TEST_SCREENSHOT_DIR) {
      await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/new-project-heading.png` });
      await taskHint.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/new-project-task-information.png` });
    }

    const typeCards = screen.locator('.np-types');
    await expect(typeCards.locator('[data-project-type-icon] svg')).toHaveCount(3);
    const firstType = typeCards.getByRole('radio').first();
    await firstType.focus();
    await page.keyboard.press('ArrowRight');
    await expect(typeCards.getByRole('radio', { name: /^Fixed Fee/ })).toBeChecked();
    await expect(screen.getByRole('group', { name: 'Project fee', exact: true })).toBeVisible();
    await saved();
    await page.keyboard.press('ArrowRight');
    await expect(typeCards.getByRole('radio', { name: /^Non-Billable/ })).toBeChecked();
    await page.keyboard.press('ArrowLeft');
    await page.keyboard.press('ArrowLeft');
    await expect(firstType).toBeChecked();
    const selectedType = typeCards.locator('label:has(input:checked)');
    assert.notEqual(await selectedType.evaluate(node => getComputedStyle(node).boxShadow), 'none');
    const iconSize = await selectedType.locator('[data-project-type-icon]').boundingBox();
    assert.equal(iconSize.width, 32);
    assert.equal(iconSize.height, 32);
    for (const name of ['np-rate-mode', 'np-visibility']) {
      const options = screen.locator(`.np-option:has(input[name="${name}"])`);
      assert.ok(await options.count() >= 2);
      for (const option of await options.all()) {
        assert.deepEqual(await option.evaluate(node => {
          const style = getComputedStyle(node);
          return [style.padding, style.borderRadius, style.borderTopWidth];
        }), ['12px', '8px', '1px']);
      }
      const chosen = options.filter({ has: page.locator('input:checked') });
      assert.notEqual(await chosen.evaluate(node => getComputedStyle(node).boxShadow), 'none');
      await options.first().getByRole('radio').focus();
      await page.keyboard.press('ArrowRight');
      await expect(options.nth(1).getByRole('radio')).toBeChecked();
      await expect(options.nth(1).getByRole('radio')).toBeFocused();
      await page.keyboard.press('ArrowLeft');
      await expect(options.first().getByRole('radio')).toBeChecked();
    }
    await saved();
    console.log('PASS: project types retain native keyboard selection and the designed icon/option-card states');

    const fieldWidth = async (id, width, numeric = false) => {
      const field = screen.locator(`#${id}`);
      assert.equal((await field.boundingBox()).width, width, `${id} matches its handoff width`);
      if (numeric) {
        assert.equal(await field.evaluate(node => getComputedStyle(node).textAlign), 'right');
        assert.match(await field.evaluate(node => getComputedStyle(node).fontFamily), /IBM Plex Mono/);
      }
    };
    await fieldWidth('np-terms', 240);
    await fieldWidth('np-po-number', 240);
    await fieldWidth('np-tax', 96, true);
    await fieldWidth('np-discount', 96, true);

    const clientPicker = page.getByRole('dialog', { name: 'Choose Client', exact: true });
    await screen.getByLabel('Client', { exact: true }).click();
    const clientSearch = clientPicker.getByRole('searchbox', { name: 'Search Client', exact: true });
    await expect(clientSearch).toBeFocused();
    await clientSearch.fill('TechStart');
    await clientPicker.getByRole('option', { name: 'TechStart Inc', exact: true }).click();
    await expect(clientPicker).not.toBeVisible();
    await expect(screen.getByLabel('Client', { exact: true })).toBeFocused();
    const currency = screen.getByRole('button', { name: 'Currency', exact: true });
    await currency.click();
    const currencyPicker = page.getByRole('listbox', { name: 'Choose Currency', exact: true });
    await expect(currencyPicker.getByRole('option', { selected: true })).toBeFocused();
    await page.keyboard.press('End');
    await expect(currencyPicker.getByRole('option', { name: 'GBP', exact: true })).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(currency).toContainText('GBP');
    await screen.getByLabel('Client', { exact: true }).click();
    await clientSearch.fill('Acme');
    await clientPicker.getByRole('option', { name: 'Acme Corp', exact: true }).click();
    await expect(currency).toContainText('GBP');
    await currency.click();
    await currencyPicker.getByRole('option', { name: /^Client default/ }).click();
    await screen.getByLabel('Client', { exact: true }).click();
    await clientSearch.fill('No such client here');
    await expect(clientPicker.getByText('No matching clients.', { exact: true })).toBeVisible();
    await clientSearch.press('Escape');
    await expect(screen.getByLabel('Client', { exact: true })).toBeFocused();
    await expect(screen.getByLabel('Client', { exact: true })).toContainText('Acme Corp');

    let releaseClients;
    await page.route('**/api/project_creation_options*', async route => {
      const response = await route.fetch();
      await new Promise(resolve => { releaseClients = resolve; });
      return route.fulfill({ response });
    });
    try {
      await screen.getByLabel('Client', { exact: true }).click();
      await expect(clientPicker.getByRole('status')).toHaveText('Loading choices…');
      await expect.poll(() => !!releaseClients).toBe(true);
      assert.equal(await clientPicker.getByRole('option').evaluateAll(options => options.length > 0 && options.every(option => option.disabled)), true);
      releaseClients();
      await clientPicker.getByRole('option', { name: 'Acme Corp', exact: true }).click();
      await readsFinished(page);
    } finally {
      releaseClients?.();
      await page.unroute('**/api/project_creation_options*');
    }
    await page.route('**/api/project_creation_options*', route => route.abort());
    await screen.getByLabel('Client', { exact: true }).click();
    await clientSearch.fill('unavailable search');
    await expect(clientPicker.getByRole('alert')).toContainText('Could not search clients');
    await expect(screen.getByLabel('Client', { exact: true })).toContainText('Acme Corp');
    await readsFinished(page);
    await page.unroute('**/api/project_creation_options*');
    await clientSearch.fill('Acme');
    await clientPicker.getByRole('option', { name: 'Acme Corp', exact: true }).click();
    console.log('PASS: searchable client selection, pending/error recovery and explicit/inherited currency');

    await screen.getByRole('button', { name: '+ New client', exact: true }).click();
    const client = page.getByRole('dialog', { name: 'New client', exact: true });
    await client.getByLabel('Client name', { exact: true }).fill('New project browser client');
    await client.getByLabel('Currency', { exact: true }).selectOption('EUR');
    await client.getByLabel('Default hourly rate', { exact: true }).fill('0');
    await client.getByRole('button', { name: /Create client/ }).click();
    await expect(client).not.toBeVisible();
    await expect(screen.getByLabel('Client', { exact: true })).toContainText('New project browser client');
    await screen.getByLabel('Project name', { exact: true }).fill('Recoverable browser project');
    await screen.getByLabel('Project code', { exact: true }).fill('BROWSER-NEW');
    const codeBox = await screen.getByLabel('Project code', { exact: true }).boundingBox();
    assert.equal(codeBox.width, 160, 'Project code uses the compact handoff width');
    await chooseDate('Start date', '1 September 2026');
    await chooseDate('End date', '15 October 2026');
    await screen.getByRole('button', { name: 'Clear End date', exact: true }).click();
    await expect(screen.getByLabel('End date', { exact: true })).toHaveText('Ends on');
    await expect(screen.getByLabel('End date', { exact: true })).toBeFocused();
    await screen.getByLabel('Tags', { exact: true }).fill('browser');
    await screen.getByLabel('Tags', { exact: true }).press('Enter');
    const tagField = screen.locator('.np-tags');
    await expect(tagField.getByRole('button', { name: 'Remove tag browser', exact: true })).toBeVisible();
    await expect(tagField.getByRole('textbox', { name: 'Tags', exact: true })).toBeVisible();
    const tagInput = screen.getByLabel('Tags', { exact: true });
    await tagInput.fill(' BROWSER , design , design ');
    await tagInput.press('Enter');
    await expect(tagField.locator('.chip')).toHaveCount(2);
    await tagInput.press('Backspace');
    await expect(tagField.locator('.chip')).toHaveCount(1);
    await tagInput.fill('keyboard');
    await tagInput.press(',');
    await tagField.getByRole('button', { name: 'Remove tag keyboard', exact: true }).focus();
    await page.keyboard.press('Enter');
    await expect(tagInput).toBeFocused();
    await expect(tagField.locator('.chip')).toHaveCount(1);
    const longTag = 'é'.repeat(51);
    await tagInput.fill(longTag);
    await tagInput.press('Enter');
    await expect(screen.getByRole('alert')).toContainText('50 characters');
    await expect(tagInput).toHaveValue(longTag);
    await expect(tagField.locator('.chip')).toHaveCount(1);
    await tagInput.fill(Array.from({ length: 50 }, (_, index) => `extra-${index}`).join(','));
    await tagInput.press('Enter');
    await expect(screen.getByRole('alert')).toContainText('50 tags');
    await expect(tagInput).not.toHaveValue('');
    await expect(tagField.locator('.chip')).toHaveCount(1);
    await tagInput.fill('');
    await expect(screen.getByRole('alert')).toHaveCount(0);
    await tagInput.fill('入力');
    await tagInput.dispatchEvent('keydown', { key: 'Enter', code: 'Enter', isComposing: true });
    await expect(tagInput).toHaveValue('入力');
    await expect(tagField.locator('.chip')).toHaveCount(1);
    await tagInput.fill('');
    await screen.getByRole('radio', { name: /^Project hourly rate/ }).check();
    await screen.getByLabel('Notes', { exact: true }).fill('W'.repeat(1000));
    await screen.locator('#np-project-rate').fill('75.25');
    await chooseField('np-budget-mode', 'Budget', 'Total project hours');
    await screen.locator('#np-budget-value').fill('120');
    await fieldWidth('np-project-rate', 120, true);
    await fieldWidth('np-budget-mode', 320);
    await fieldWidth('np-budget-value', 160, true);
    await expect(screen.locator('.np-option:has(input[name="np-rate-mode"]:checked) #np-project-rate')).toBeVisible();
    const budgetTypeBox = await screen.locator('#np-budget-mode').boundingBox();
    const budgetAmountBox = await screen.locator('#np-budget-value').boundingBox();
    assert.ok(Math.abs((budgetTypeBox.y + budgetTypeBox.height / 2) -
      (budgetAmountBox.y + budgetAmountBox.height / 2)) <= 1, 'Budget type and amount align in a desktop row');
    await expect(screen.getByRole('checkbox', { name: /^Email me/ })).toBeDisabled();
    const addPerson = screen.locator('#np-add-person');
    assert.equal(await addPerson.evaluate(node => node.tagName), 'BUTTON', 'Team uses the shared searchable selector');
    await addPerson.click();
    const peoplePicker = page.getByRole('dialog', { name: 'Choose teammate', exact: true });
    const peopleSearch = peoplePicker.getByRole('searchbox', { name: 'Search teammate', exact: true });
    await expect(peopleSearch).toBeFocused();
    await peopleSearch.fill('No matching teammate');
    await expect(peoplePicker.getByRole('status')).toHaveText('No matching teammates.');
    await readsFinished(page);
    let releasePeople;
    await page.route('**/api/project_creation_options*', async route => {
      const response = await route.fetch();
      await new Promise(resolve => { releasePeople = resolve; });
      return route.fulfill({ response });
    });
    try {
      await peopleSearch.fill('Admin');
      await expect(peoplePicker.getByRole('status')).toHaveText('Loading choices…');
      await expect.poll(() => !!releasePeople).toBe(true);
      assert.equal(await peoplePicker.getByRole('option').evaluateAll(items => items.every(item => item.disabled)), true);
      releasePeople();
      await expect(peoplePicker.getByRole('option', { name: 'Admin User', exact: true })).toBeEnabled();
      await peopleSearch.press('ArrowDown');
      await expect(peoplePicker.getByRole('option', { name: 'Admin User', exact: true })).toBeFocused();
      await page.keyboard.press('Enter');
      await expect(peoplePicker).not.toBeVisible();
      await expect(addPerson).toBeFocused();
      await readsFinished(page);
    } finally {
      releasePeople?.();
      await page.unroute('**/api/project_creation_options*');
    }
    await addPerson.click();
    await expect(peoplePicker.getByRole('option', { name: 'Admin User', exact: true })).toBeDisabled();
    await readsFinished(page);
    await page.route('**/api/project_creation_options*', route => route.abort());
    await peopleSearch.fill('Unavailable teammate search');
    await expect(peoplePicker.getByRole('alert')).toContainText('Could not search teammates');
    await expect(screen.getByRole('button', { name: 'Remove Admin User from project', exact: true })).toHaveCount(1);
    await readsFinished(page);
    await page.unroute('**/api/project_creation_options*');
    await peoplePicker.getByRole('button', { name: 'Retry teammate search', exact: true }).click();
    await expect(peoplePicker.getByRole('status')).toHaveText('No matching teammates.');
    await peopleSearch.press('Escape');
    await expect(addPerson).toBeFocused();
    await screen.getByRole('button', { name: 'Remove Admin User from project', exact: true }).click();
    await screen.getByRole('button', { name: 'Add everyone', exact: true }).click();
    await expect(screen.getByLabel('Project name', { exact: true })).toBeEnabled();
    await expect(screen.getByRole('checkbox', { name: /manages this project/ }).first()).toBeVisible();
    await screen.getByRole('checkbox', { name: /manages this project/ }).first().check();
    const tasksSection = screen.getByRole('region', { name: 'Tasks', exact: true });
    const taskSearch = tasksSection.getByRole('textbox', { name: 'Find or create a task', exact: true });
    const addTask = tasksSection.getByRole('button', { name: 'Add task', exact: true });
    await readsFinished(page);
    let releaseTasks;
    await page.route('**/api/project_creation_options*', async route => {
      const response = await route.fetch();
      await new Promise(resolve => { releaseTasks = resolve; });
      return route.fulfill({ response });
    });
    try {
      await taskSearch.fill('Development');
      await expect(tasksSection.getByRole('status')).toHaveText('Searching tasks…');
      await expect.poll(() => !!releaseTasks).toBe(true);
      await expect(addTask).toBeDisabled();
      await expect(tasksSection.getByRole('button', { name: 'Design', exact: true })).toHaveCount(0);
      await taskSearch.press('Enter');
      await expect(tasksSection.getByRole('button', { name: 'Remove task Development', exact: true })).toHaveCount(0);
      releaseTasks();
      await expect(addTask).toBeEnabled();
      await page.unroute('**/api/project_creation_options*');
      await taskSearch.press('Enter');
      await expect(tasksSection.getByRole('button', { name: 'Remove task Development', exact: true })).toHaveCount(1);
      await readsFinished(page);
    } finally {
      releaseTasks?.();
      await page.unroute('**/api/project_creation_options*');
    }
    await expect(tasksSection.getByRole('button', { name: 'Development', exact: true })).toBeDisabled();
    await page.route('**/api/project_creation_options*', route => route.abort());
    await taskSearch.fill('Browser custom task');
    await expect(tasksSection.getByRole('alert')).toContainText('Could not search tasks');
    await expect(addTask).toBeDisabled();
    await taskSearch.press('Enter');
    await expect(taskSearch).toHaveValue('Browser custom task');
    await expect(tasksSection.getByRole('button', { name: 'Remove task Browser custom task', exact: true })).toHaveCount(0);
    await readsFinished(page);
    await page.unroute('**/api/project_creation_options*');
    await tasksSection.getByRole('button', { name: 'Retry task search', exact: true }).click();
    await expect(tasksSection.getByRole('status')).toHaveText('No matching tasks. Add this name as a new task.');
    await addTask.click();
    await expect(taskSearch).toBeFocused();
    assert.equal((await taskSearch.locator('..').boundingBox()).height, 40, 'Task entry matches the compact design control');
    console.log('PASS: task and teammate searches handle pending results, keyboard selection, duplicates and retry');
    const teamSection = screen.getByRole('region', { name: 'Team', exact: true });
    const costInput = teamSection.locator('input[id^="np-cost-rate-"]').first();
    assert.equal((await costInput.boundingBox()).width, 120, 'Project cost overrides are compact');
    await expect(teamSection.locator('.avatar').first()).toHaveCSS('width', '30px');
    await expect(teamSection).toContainText('Project manager');
    await expect(screen.locator('.np-assignment-row')).toHaveCount(3);
    for (const row of await screen.locator('.np-assignment-row').all()) {
      assert.ok((await row.boundingBox()).height <= 72, 'Default assignment rows remain compact on desktop');
    }
    await screen.getByRole('radio', { name: /^Person hourly rate/ }).check();
    await chooseField('np-budget-mode', 'Budget', 'Hours per person');
    await teamSection.getByLabel('Billable rate for Admin User (EUR/h)', { exact: true }).fill('0');
    await teamSection.getByLabel('Budget hours for Admin User', { exact: true }).fill('12:30');
    await costInput.fill('0');
    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      await addPerson.click();
      await expect(peoplePicker.getByRole('option', { name: 'Admin User', exact: true })).toBeDisabled();
      const pickerBox = await peoplePicker.boundingBox();
      assert.ok(pickerBox.x >= 0 && pickerBox.x + pickerBox.width <= width, 'Teammate search fits the viewport');
      await peopleSearch.press('Escape');
      await expect(addPerson).toBeFocused();
      await readsFinished(page);
      for (const input of await teamSection.locator('.np-row-controls input').all()) {
        assert.equal((await input.boundingBox()).width, 120);
      }
      const row = teamSection.locator('.np-assignment-row');
      assert.ok(await row.evaluate(node => node.scrollWidth <= node.clientWidth), 'All member rates/budget fit the row');
      if (process.env.HORAE_TEST_SCREENSHOT_DIR) {
        await row.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/new-project-team-row-${width}.png` });
      }
    }
    await screen.getByRole('radio', { name: /^Task hourly rate/ }).check();
    await chooseField('np-budget-mode', 'Budget', 'Hours per task');
    const taskRate = screen.getByLabel('Hourly rate for Browser custom task (EUR)', { exact: true });
    await taskRate.fill('0');
    await screen.getByLabel('Budget hours for Browser custom task', { exact: true }).fill('4');
    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      for (const input of await screen.getByRole('region', { name: 'Tasks', exact: true }).locator('.np-row-controls input').all()) {
        assert.equal((await input.boundingBox()).width, 120);
      }
      const row = screen.locator('.np-assignment-row').filter({ has: page.getByLabel('Hourly rate for Browser custom task (EUR)', { exact: true }) });
      assert.ok(await row.evaluate(node => node.scrollWidth <= node.clientWidth), 'Task rate/budget/access fit the row');
      if (process.env.HORAE_TEST_SCREENSHOT_DIR) {
        await row.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/new-project-task-row-${width}.png` });
      }
    }
    await screen.getByRole('radio', { name: /^Project hourly rate/ }).check();
    await chooseField('np-budget-mode', 'Budget', 'Total project hours');
    await screen.getByRole('button', { name: /^Access for Browser custom task:/ }).click();
    const access = page.getByRole('dialog', { name: 'Who can track to this task?' });
    await access.getByRole('radio', { name: 'Only selected people', exact: true }).check();
    await access.getByRole('checkbox').first().check();
    await access.getByRole('button', { name: 'Apply access', exact: true }).click();
    await expect(access).not.toBeVisible();
    await chooseField('np-terms', 'Payment terms', 'Custom days');
    await screen.getByLabel('Days until payment is due', { exact: true }).fill('21');
    await screen.getByLabel('Tax (%)', { exact: true }).fill('21');
    await screen.getByRole('button', { name: 'Add a second tax', exact: true }).click();
    await screen.getByLabel('Second tax name', { exact: true }).fill('Local tax');
    await screen.getByLabel('Second tax (%)', { exact: true }).fill('1.5');
    await fieldWidth('np-second-tax-name', 160);
    await fieldWidth('np-second-tax', 96, true);
    await screen.getByRole('button', { name: 'Remove second tax', exact: true }).focus();
    await page.keyboard.press('Enter');
    await expect(screen.getByLabel('Tax (%)', { exact: true })).toBeFocused();
    await expect(screen.getByLabel('Second tax name', { exact: true })).toHaveCount(0);
    await screen.getByRole('button', { name: 'Add a second tax', exact: true }).click();
    await screen.getByLabel('Second tax name', { exact: true }).fill('Local tax');
    await screen.getByLabel('Second tax (%)', { exact: true }).fill('1.5');
    await saved();
    await readsFinished(page);
    // Simulate a catalog page that does not contain the saved selections. The
    // selected-ID endpoint still reads the real, isolated database.
    await page.route('**/api/project_creation_options*', async route => {
      const response = await route.fetch();
      const options = await response.json();
      return route.fulfill({ response, json: { ...options, tasks: [], people: [], more_tasks: true, more_people: true } });
    });
    const selectedReady = page.waitForResponse(response => response.url().includes('/api/project_creation_selection') && response.status() === 200);
    await page.reload();
    const resolved = await (await selectedReady).json();
    assert.deepEqual(resolved.tasks.map(task => task.name), ['Development']);
    assert.deepEqual(resolved.people.map(person => person.name), ['Admin User']);
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Recoverable browser project');
    await expect(screen.getByLabel('Start date', { exact: true })).toHaveText('01 Sep 2026');
    await expect(screen.getByLabel('End date', { exact: true })).toHaveText('Ends on');
    // Keyboard selection, Escape and reopening after browsing a different month.
    const startDate = screen.getByLabel('Start date', { exact: true });
    const startCalendar = page.getByRole('dialog', { name: 'Choose Start date', exact: true });
    await startDate.focus();
    await startDate.press('Enter');
    await expect(startCalendar.getByRole('button', { name: '1 September 2026', exact: true })).toBeFocused();
    await startCalendar.getByRole('button', { name: 'Next month' }).click();
    await page.keyboard.press('Escape');
    await expect(startCalendar).not.toBeVisible();
    await expect(startDate).toBeFocused();
    await startDate.press('Enter');
    await expect(startCalendar.getByRole('button', { name: '1 September 2026', exact: true })).toBeFocused();
    await page.keyboard.press('Tab');
    await expect(startCalendar.getByRole('button', { name: '2 September 2026', exact: true })).toBeFocused();
    await page.keyboard.press('Shift+Tab');
    await page.keyboard.press('Enter');
    await expect(startCalendar).not.toBeVisible();
    await startDate.click();
    await screen.getByLabel('End date', { exact: true }).click();
    await expect(startCalendar).not.toBeVisible();
    const endCalendar = page.getByRole('dialog', { name: 'Choose End date', exact: true });
    await expect(endCalendar).toBeVisible();
    await endCalendar.getByRole('button', { name: 'Today', exact: true }).focus();
    await page.keyboard.press('Tab');
    await expect(endCalendar).not.toBeVisible();
    await expect(screen.getByRole('button', { name: 'Remove tag browser', exact: true })).toBeFocused();
    await expect(screen.locator('#np-project-rate')).toHaveValue('75.25');
    await expect(screen.locator('#np-budget-value')).toHaveValue('120');
    await expect(costInput).toHaveValue('0');
    await screen.getByRole('radio', { name: /^Person hourly rate/ }).check();
    await chooseField('np-budget-mode', 'Budget', 'Hours per person');
    await expect(teamSection.getByLabel('Billable rate for Admin User (EUR/h)', { exact: true })).toHaveValue('0');
    await expect(teamSection.getByLabel('Budget hours for Admin User', { exact: true })).toHaveValue('12:30');
    await screen.getByRole('radio', { name: /^Task hourly rate/ }).check();
    await chooseField('np-budget-mode', 'Budget', 'Hours per task');
    await expect(taskRate).toHaveValue('0');
    await expect(screen.getByLabel('Budget hours for Browser custom task', { exact: true })).toHaveValue('4');
    await screen.getByRole('radio', { name: /^Project hourly rate/ }).check();
    await chooseField('np-budget-mode', 'Budget', 'Total project hours');
    await expect(screen.getByLabel('Days until payment is due', { exact: true })).toHaveValue('21');
    await expect(screen.getByLabel('Second tax name', { exact: true })).toHaveValue('Local tax');
    await expect(screen.getByRole('button', { name: /^Access for Browser custom task:/ })).not.toContainText('Everyone');
    await expect(screen.getByRole('button', { name: 'Remove task Development', exact: true })).toBeVisible();
    await expect(screen.getByRole('checkbox', { name: 'Admin User manages this project', exact: true })).toBeChecked();
    await expect(screen).not.toContainText('Unavailable task');
    await expect(screen).not.toContainText('Unavailable teammate');
    await readsFinished(page);
    await page.unroute('**/api/project_creation_options*');
    console.log('PASS: real client, team, task restrictions, billing and invoice defaults survive reload');

    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      for (const [id, invalid] of [
        ['np-terms-days', '366'], ['np-po-number', 'P'.repeat(201)],
        ['np-tax', '1.001'], ['np-second-tax-name', ''],
        ['np-second-tax', '-1'], ['np-discount', '101'],
      ]) {
        const field = screen.locator(`#${id}`);
        const original = await field.inputValue();
        await field.fill(invalid);
        await saved();
        await screen.getByRole('button', { name: 'Save project', exact: true }).focus();
        await page.keyboard.press('Enter');
        await expect(draftStatus).toHaveText('Changes need attention');
        await expect(field).toHaveAttribute('aria-invalid', 'true');
        await expect(field).toHaveAttribute('aria-describedby', 'np-invoice-field-error');
        await expect(field).toBeFocused();
        await expect(field).toHaveValue(invalid);
        await expect(screen.getByRole('button', { name: 'Save project', exact: true })).toBeEnabled();
        if (width === 390 && id === 'np-tax') {
          const rejectedAgain = page.waitForResponse(response => response.url().includes('/api/finalize_project_draft') && response.status() === 400);
          await screen.getByRole('button', { name: 'Save project', exact: true }).focus();
          await page.keyboard.press('Enter');
          await (await rejectedAgain).finished();
          await expect(field).toBeFocused();
          await expect(field).toHaveValue(invalid);
        }
        await expect(screen.locator('#np-form-error-message')).not.toBeEmpty();
        await expect(screen.locator('#np-invoice-field-error')).toHaveText(await screen.locator('#np-form-error-message').innerText());
        await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Recoverable browser project');
        const bounds = await field.boundingBox();
        const footer = await screen.locator('.np-footer').boundingBox();
        assert.ok(bounds.y >= 0 && bounds.y + bounds.height <= footer.y, `${id} error is not obscured at ${width}`);
        const errorBounds = await screen.locator('#np-invoice-field-error').boundingBox();
        assert.ok(errorBounds.y >= 0 && errorBounds.y + errorBounds.height <= footer.y, `${id} error message is visible at ${width}`);
        if (process.env.HORAE_TEST_SCREENSHOT_DIR && id === 'np-second-tax') {
          await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/new-project-field-error-${width}.png` });
        }
        if (width === 1440 && id === 'np-discount') {
          await readsFinished(page);
          await screen.getByRole('button', { name: 'Cancel', exact: true }).focus();
          await page.keyboard.press('Enter');
          await expect(page).toHaveURL(`${base}/projects`);
          await page.getByRole('link', { name: 'New project', exact: true }).click();
          await expect(field).toHaveValue(invalid);
          await field.fill(original);
          await saved();
          continue;
        }
        await field.fill(original);
        await screen.getByRole('button', { name: 'Retry request', exact: true }).focus();
        await page.keyboard.press('Enter');
        await saved();
        await expect(screen.locator('[aria-invalid="true"]')).toHaveCount(0);
        await expect(page).toHaveURL(`${base}/projects/new`);
      }
    }
    console.log('PASS: all invoice-default errors identify and focus their field, preserve input and recover at three widths');

    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      for (const [id, invalid] of [
        ['np-name', 'N'.repeat(201)], ['np-code', 'C'.repeat(101)],
        ['np-notes', 'Private context '.repeat(667)], ['np-end', '31 Aug 2026'],
      ]) {
        const field = screen.locator(`#${id}`);
        const original = id === 'np-end' ? null : await field.inputValue();
        if (id === 'np-end') await chooseDate('End date', '31 August 2026');
        else await field.fill(invalid);
        await saved();
        await screen.getByRole('button', { name: 'Save project', exact: true }).focus();
        await page.keyboard.press('Enter');
        await expect(draftStatus).toHaveText('Changes need attention');
        await expect(field).toHaveAttribute('aria-invalid', 'true');
        await expect(field).toHaveAttribute('aria-describedby', 'np-basic-field-error');
        await expect(field).toBeFocused();
        if (id === 'np-end') await expect(field).toHaveText(invalid);
        else await expect(field).toHaveValue(invalid);
        const message = screen.locator('#np-basic-field-error');
        await expect(message).toHaveText(await screen.locator('#np-form-error-message').innerText());
        const footer = await screen.locator('.np-footer').boundingBox();
        for (const item of [field, message]) {
          const bounds = await item.boundingBox();
          assert.ok(bounds.y >= 0 && bounds.y + bounds.height <= footer.y, `${id} and its error remain visible at ${width}`);
        }
        if (id === 'np-end') await screen.getByRole('button', { name: 'Clear End date', exact: true }).click();
        else await field.fill(original);
        await screen.getByRole('button', { name: 'Retry request', exact: true }).focus();
        await page.keyboard.press('Enter');
        await saved();
        await expect(screen.locator('[aria-invalid="true"]')).toHaveCount(0);
        await expect(page).toHaveURL(`${base}/projects/new`);
      }
    }
    console.log('PASS: basic-field rejections retain values, link visible errors and restore focus at three widths');

    await readsFinished(page);
    await screen.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    const writesBeforeFailedResume = draftWrites;
    await page.route('**/api/load_project_draft*', route => route.abort());
    await page.getByRole('link', { name: 'New project', exact: true }).click();
    await expect(page.getByRole('alert')).toContainText('Could not load project settings');
    await expect(screen).toHaveCount(0);
    assert.equal(draftWrites, writesBeforeFailedResume);
    await page.unroute('**/api/load_project_draft*');
    await page.getByRole('button', { name: 'Retry', exact: true }).click();
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Recoverable browser project');
    await expect(screen.locator('#np-code')).toHaveValue('BROWSER-NEW');
    await expect(screen.locator('#np-start')).toHaveText('01 Sep 2026');
    await saved();
    assert.equal(draftWrites, writesBeforeFailedResume, 'Resuming after failure must not rewrite the acknowledged draft');
    console.log('PASS: failed resume and retry preserve the existing draft without a replacement write');

    const wideTag = 'W'.repeat(50);
    await tagInput.fill(wideTag);
    await tagInput.press('Enter');
    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      await screen.getByRole('button', { name: 'Back to Projects', exact: true }).focus();
      let reachedSave = false;
      for (let step = 0; step < 120; step++) {
        await page.keyboard.press('Tab');
        const focus = await page.evaluate(() => {
          const active = document.activeElement;
          const footer = document.querySelector('.np-footer');
          const box = active.getBoundingClientRect();
          return { name: active.getAttribute('aria-label') || active.id || active.textContent,
            save: active === [...footer.querySelectorAll('button')].find(button => button.textContent === 'Save project'),
            inForm: !!active.closest('.np-page') && !footer.contains(active),
            top: box.top, bottom: box.bottom, footerTop: footer.getBoundingClientRect().top,
            scrollTop: document.querySelector('.np-scroll')?.getBoundingClientRect().top || 0 };
        });
        if (focus.save) { reachedSave = true; break; }
        assert.ok(focus.inForm, `Tab stays in the form until its actions: ${focus.name}`);
        assert.ok(focus.top >= focus.scrollTop - 1 && focus.bottom <= focus.footerTop + 1,
          `Focused ${focus.name} is not obscured at ${width}px: ${JSON.stringify(focus)}`);
      }
      assert.ok(reachedSave, `Keyboard reaches Save project at ${width}px`);
      assert.equal(await screen.locator('.np-footer').evaluate(node => getComputedStyle(node).paddingLeft),
        width < 900 ? '20px' : '40px');
      const frame = await screen.locator('.np-footer').evaluate(footer => ({
        footerBottom: footer.getBoundingClientRect().bottom,
        scrollBottom: document.querySelector('.np-scroll').getBoundingClientRect().bottom,
        footerTop: footer.getBoundingClientRect().top,
        documentHeight: document.documentElement.scrollHeight,
      }));
      assert.equal(frame.footerBottom, 900);
      assert.equal(frame.documentHeight, 900);
      assert.equal(frame.scrollBottom, frame.footerTop, 'Actions do not overlap the scrolling form');
      await fieldWidth('np-project-rate', 120, true);
      await fieldWidth('np-budget-value', 160, true);
      await fieldWidth('np-terms', 240);
      await fieldWidth('np-po-number', 240);
      await fieldWidth('np-tax', 96, true);
      await fieldWidth('np-second-tax-name', 160);
      await fieldWidth('np-second-tax', 96, true);
      await fieldWidth('np-discount', 96, true);
      if (width === 390) {
        const rateLabel = await screen.locator('.np-option label:has(input[name="np-rate-mode"]:checked)').boundingBox();
        const rateField = await screen.locator('#np-project-rate').boundingBox();
        assert.ok(rateField.y >= rateLabel.y + rateLabel.height,
          'At mobile width the rate moves below its label instead of crushing the description');
      }
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true,
        `New project fits at ${width}px`);
      await screen.getByLabel('End date', { exact: true }).click();
      const calendar = page.getByRole('dialog', { name: 'Choose End date', exact: true });
      await expect(calendar).toBeVisible();
      await expect(calendar.locator('.dp-day.picked')).toBeFocused();
      const bounds = await calendar.boundingBox();
      assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= width, `Calendar fits at ${width}px`);
      if (process.env.HORAE_TEST_SCREENSHOT_DIR) {
        await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/new-project-calendar-${width}.png` });
      }
      await page.keyboard.press('Escape');
      await expect(calendar).not.toBeVisible();
      for (const name of ['Client', 'Currency']) {
        const trigger = screen.getByRole('button', { name, exact: true });
        await trigger.click();
        const panel = page.getByRole('dialog', { name: `Choose ${name}`, exact: true });
        await expect(panel).toBeVisible();
        const box = await panel.boundingBox();
        assert.ok(box.x >= 0 && box.x + box.width <= width, `${name} fits at ${width}px`);
        assert.ok(box.y >= 0 && box.y + box.height <= 901, `${name} fits vertically`);
        if (process.env.HORAE_TEST_SCREENSHOT_DIR) {
          await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/new-project-${name.toLowerCase()}-${width}.png` });
        }
        await page.keyboard.press('Escape');
        await expect(panel).not.toBeVisible();
        await expect(trigger).toBeFocused();
      }
      // Screenshots restore caret styling by leaving empty style attributes.
      await expect(screen.locator('[style]:not([style=""])')).toHaveCount(0);
      if (process.env.HORAE_TEST_SCREENSHOT_DIR) {
        await page.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/new-project-${width}.png`, fullPage: true });
        await screen.locator('.np-row:has(.np-types)').screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/new-project-billing-${width}.png` });
        await screen.getByRole('region', { name: 'Invoice defaults', exact: true }).screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/new-project-invoice-defaults-${width}.png` });
      }
    }
    const collapse = page.getByRole('button', { name: 'Collapse sidebar', exact: true });
    await collapse.click();
    await expect(page.locator('.app-sidebar')).toHaveCSS('width', '68px');
    assert.equal((await screen.locator('.np-footer').boundingBox()).x, 68);
    await collapse.click();
    await page.setViewportSize({ width: 390, height: 320 });
    const mobileOpen = page.getByRole('button', { name: 'Open navigation', exact: true });
    await mobileOpen.click();
    const sidebar = page.locator('.app-sidebar');
    await expect(sidebar).toBeVisible();
    await sidebar.locator('.sidebar-footer').scrollIntoViewIfNeeded();
    const account = await sidebar.locator('.sidebar-footer').boundingBox();
    assert.ok(account.y >= 0 && account.y + account.height <= 320, 'Short-screen navigation remains reachable');
    await page.keyboard.press('Escape');
    await expect(sidebar).toBeHidden();
    await expect(mobileOpen).toBeFocused();
    assert.equal(await page.evaluate(() => document.documentElement.scrollHeight), 320);
    const clientTrigger = screen.getByRole('button', { name: 'Client', exact: true });
    await clientTrigger.click();
    await expect(clientSearch).toBeFocused();
    const shortPanel = await clientPicker.boundingBox();
    assert.ok(shortPanel.y >= 0 && shortPanel.y + shortPanel.height <= 320, 'Choices fit the short viewport');
    await clientSearch.press('Escape');
    // Leave room for the persistent actions and a trigger in the lower half.
    await page.setViewportSize({ width: 390, height: 420 });
    await clientTrigger.evaluate(element => {
      const scroller = element.closest('.np-scroll');
      const target = Math.min(innerHeight / 2 + 20, scroller.getBoundingClientRect().bottom - element.offsetHeight);
      scroller.scrollBy(0, element.getBoundingClientRect().top - target);
    });
    await clientTrigger.focus();
    await clientTrigger.press('Enter');
    await expect(clientPicker.getByRole('option', { name: 'New project browser client', exact: true })).toBeEnabled();
    assert.ok((await clientPicker.boundingBox()).y < (await clientTrigger.boundingBox()).y, 'Short viewport opens the client choices above the field');
    await clientSearch.fill('TechStart');
    await expect(clientPicker.getByRole('option', { name: 'TechStart Inc', exact: true })).toBeEnabled();
    await expect.poll(async () => {
      const anchor = await clientTrigger.boundingBox(), panel = await clientPicker.boundingBox();
      return Math.abs(anchor.y - (panel.y + panel.height) - 4);
    }).toBeLessThanOrEqual(1);
    await clientSearch.press('Escape');
    await page.setViewportSize({ width: 1440, height: 900 });
    await tagField.getByRole('button', { name: `Remove tag ${wideTag}`, exact: true }).click();
    await screen.getByRole('radio', { name: /^Fixed Fee/ }).check();
    await fieldWidth('np-fee-amount', 200, true);
    await screen.getByRole('radio', { name: 'Monthly', exact: true }).check();
    await fieldWidth('np-monthly-day', 200);
    await chooseField('np-monthly-day', 'Monthly invoice day', 'Last day of the month');
    await screen.getByRole('radio', { name: 'Milestones', exact: true }).check();
    await screen.getByRole('button', { name: 'Add milestone', exact: true }).click();
    await screen.getByLabel('Milestone name', { exact: true }).fill('Delivery');
    await screen.getByLabel('Due date', { exact: true }).fill('2026-10-01');
    await screen.getByLabel('Amount (EUR)', { exact: true }).fill('1000.25');
    const milestoneRow = screen.getByRole('group', { name: 'Milestone Delivery', exact: true });
    const milestoneAmount = milestoneRow.getByLabel('Amount (EUR)', { exact: true });
    assert.equal((await milestoneRow.getByLabel('Due date', { exact: true }).boundingBox()).width, 160);
    assert.equal((await milestoneAmount.boundingBox()).width, 140);
    const removeMilestone = milestoneRow.getByRole('button', { name: 'Remove milestone Delivery', exact: true });
    assert.equal((await removeMilestone.boundingBox()).width, 40);
    const rowBoxes = await Promise.all([milestoneRow.getByLabel('Milestone name', { exact: true }), milestoneRow.getByLabel('Due date', { exact: true }), milestoneAmount, removeMilestone].map(field => field.boundingBox()));
    assert.ok(rowBoxes.every(box => Math.abs(box.y + box.height / 2 - rowBoxes[0].y - rowBoxes[0].height / 2) <= 1), 'Milestone fields and removal align in one desktop row');
    await milestoneRow.getByLabel('Milestone name', { exact: true }).focus();
    await page.keyboard.press('Tab');
    await expect(milestoneRow.getByLabel('Due date', { exact: true })).toBeFocused();
    for (let step = 0; step < 8; step++) {
      await page.keyboard.press('Tab');
      if (await milestoneAmount.evaluate(node => node === document.activeElement)) break;
    }
    await expect(milestoneAmount).toBeFocused();
    await page.keyboard.press('Tab');
    await expect(removeMilestone).toBeFocused();
    await screen.getByRole('button', { name: 'Add milestone', exact: true }).click();
    const extraMilestone = screen.getByRole('group', { name: 'New milestone', exact: true });
    await extraMilestone.getByLabel('Amount (EUR)', { exact: true }).fill('0.75');
    await expect(screen).toContainText('Total EUR 1001.00');
    for (const width of [390, 768, 769, 900, 1180, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      assert.ok(await milestoneRow.evaluate(node => node.scrollWidth <= node.clientWidth), 'Milestone fields fit narrow screens');
      assert.ok(await screen.evaluate(node => node.scrollWidth <= node.clientWidth), 'Milestones do not widen the form');
      const nameWidth = (await milestoneRow.getByLabel('Milestone name', { exact: true }).boundingBox()).width;
      assert.ok(nameWidth >= 120, `Milestone name stays readable at viewport ${width}: ${nameWidth}px`);
      await milestoneRow.getByLabel('Milestone name', { exact: true }).focus();
      for (const field of [milestoneRow.getByLabel('Due date', { exact: true }), milestoneAmount, removeMilestone]) {
        // Native date fields contain several keyboard segments; focus each control
        // explicitly here, then assert the scrolling panel exposes its whole box.
        await field.focus();
        const fieldBox = await field.boundingBox(), footerBox = await screen.locator('footer').boundingBox();
        assert.ok(fieldBox.y >= 0 && fieldBox.y + fieldBox.height <= footerBox.y + 1, 'Focused milestone control stays above actions');
      }
      if (process.env.HORAE_TEST_SCREENSHOT_DIR) {
        await milestoneRow.screenshot({ path: `${process.env.HORAE_TEST_SCREENSHOT_DIR}/new-project-milestone-${width}.png` });
      }
    }
    await extraMilestone.getByRole('button', { name: 'Remove milestone', exact: true }).focus();
    await page.keyboard.press('Enter');
    await expect(screen.getByRole('button', { name: 'Add milestone', exact: true })).toBeFocused();
    await expect(milestoneAmount).toHaveValue('1000.25');
    await expect(screen).toContainText('Total EUR 1000.25');
    await saved();
    await readsFinished(page);
    await page.reload();
    await expect(milestoneRow.getByLabel('Due date', { exact: true })).toHaveValue('2026-10-01');
    await expect(milestoneAmount).toHaveValue('1000.25');
    await screen.getByRole('radio', { name: 'Monthly', exact: true }).check();
    await expect(screen.locator('#np-monthly-day')).toContainText('Last day of the month');
    await screen.getByRole('radio', { name: 'Milestones', exact: true }).check();
    await screen.getByRole('radio', { name: /^Non-Billable/ }).check();
    await expect(screen.getByRole('heading', { name: 'Invoice defaults', exact: true })).toHaveCount(0);
    await screen.getByRole('radio', { name: /^Time & Materials/ }).check();
    await expect(screen.locator('#np-project-rate')).toHaveValue('75.25');
    await saved();
    await screen.getByRole('button', { name: 'Cancel', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await page.getByRole('link', { name: 'New project', exact: true }).click();
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Recoverable browser project');
    console.log('PASS: responsive form, conditional panels and Cancel preserve the draft');

    let release;
    let hold = true;
    await page.route('**/api/save_project_draft*', async route => {
      const response = await route.fetch();
      if (hold) await new Promise(resolve => { release = resolve; });
      return route.fulfill({ response });
    });
    try {
      await screen.getByLabel('Project name', { exact: true }).fill('Older snapshot');
      await expect.poll(() => !!release).toBe(true);
      await screen.getByLabel('Project name', { exact: true }).fill('Edited during save');
      await expect(draftStatus).toHaveText('Saving draft…');
      hold = false;
      release();
      await saved();
      await readsFinished(page);
      await page.reload();
      await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Edited during save');
    } finally {
      hold = false;
      if (release) release();
      await page.unroute('**/api/save_project_draft*');
    }
    console.log('PASS: edits made while a save is pending are saved after its acknowledgement');

    // Commit the first request, then lose its response. A retry must acknowledge
    // that same snapshot before saving edits made during the uncertain outcome.
    let lost;
    let retry;
    await page.route('**/api/save_project_draft*', async route => {
      if (!lost) {
        lost = route.request().postData();
        await route.fetch();
        return route.abort();
      }
      retry ??= route.request().postData();
      return route.continue();
    });
    await screen.getByLabel('Project name', { exact: true }).fill('Acknowledgement lost');
    await expect(screen.getByRole('alert')).toBeVisible();
    await expect(draftStatus).toHaveText('Changes need attention');
    await screen.getByLabel('Project name', { exact: true }).fill('Recovered latest edit');
    await expect(screen.getByRole('button', { name: 'Save project', exact: true })).toBeDisabled();
    await expect(screen.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
    await screen.getByRole('button', { name: 'Retry request', exact: true }).click();
    await saved();
    assert.equal(retry, lost, 'The first retry reuses the exact uncertain request');
    await readsFinished(page);
    await page.reload();
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('Recovered latest edit');
    await page.unroute('**/api/save_project_draft*');
    console.log('PASS: lost acknowledgement retries the same request and preserves newer edits');

    await screen.getByLabel('Tax (%)', { exact: true }).fill('101');
    await screen.getByRole('button', { name: 'Save project', exact: true }).click();
    await expect(screen.getByLabel('Tax (%)', { exact: true })).toHaveAttribute('aria-invalid', 'true');
    await screen.getByLabel('Tax (%)', { exact: true }).fill('21');
    // A definite field rejection can be corrected and explicitly submitted;
    // the subsequent uncertain commit must still require its original retry.
    let created;
    await page.route('**/api/finalize_project_draft*', async route => {
      const response = await route.fetch();
      const id = await response.json();
      assert.equal(response.status(), 200, JSON.stringify(id));
      if (!created) {
        created = id;
        return route.abort();
      }
      assert.equal(id, created, 'Retry returns the already-created project');
      return route.fulfill({ response });
    });
    await screen.getByRole('button', { name: 'Save project', exact: true }).click();
    await expect(screen.getByRole('alert')).toBeVisible();
    await expect(screen.getByLabel('Project name', { exact: true })).toBeDisabled();
    await expect(screen.getByRole('button', { name: 'Saving project…', exact: true })).toBeDisabled();
    await expect(screen.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
    await expect(screen.getByRole('button', { name: 'Back to Projects', exact: true })).toBeDisabled();
    await screen.getByRole('button', { name: 'Retry request', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects/${created}`);
    await expect(page.getByRole('heading', { name: 'Project', exact: true })).toBeVisible();
    const basics = page.getByRole('region', { name: 'Project details', exact: true });
    await expect(basics).toContainText('Recovered latest edit');
    await expect(basics).toContainText('BROWSER-NEW');
    await expect(basics).toContainText('New project browser client');
    await expect(basics).toContainText('01 Sep 2026');
    await expect(basics.locator('.chip')).toHaveText(['browser']);
    await expect(basics).toContainText('W'.repeat(1000));
    for (const width of [390, 768, 1440]) {
      await page.setViewportSize({ width, height: 900 });
      assert.equal(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1), true,
        `Saved project details fit at ${width}px even with unbroken notes`);
      assert.equal(await basics.evaluate(element => element.scrollWidth <= element.clientWidth + 1), true);
    }
    await readsFinished(page);
    const budgetRead = page.waitForResponse(response => response.url().includes('/api/list_project_budget_progress') && response.status() === 200);
    await page.getByRole('link', { name: 'Projects', exact: true }).click();
    const budgets = await (await budgetRead).json();
    const budget = budgets.find(row => row.project_id === created);
    assert.ok(budget, 'The real created project has configured progress');
    assert.deepEqual([budget.kind, budget.budget, budget.consumed, budget.period_key], ['hours', 7200, 0, 'lifetime']);
    const createdRow = page.locator('.proj-row').filter({ has: page.getByRole('link', { name: '[BROWSER-NEW] Recovered latest edit', exact: true }) });
    await expect(createdRow.getByRole('progressbar')).toHaveAttribute('value', '0');
    await expect(createdRow).toContainText('120h');
    await expect(createdRow).toContainText('Total tracked: 0h');
    await page.getByRole('checkbox', { name: 'Select all visible projects', exact: true }).click();
    await page.getByRole('button', { name: /^All tags/ }).click();
    await page.getByRole('menuitem', { name: 'browser', exact: true }).click();
    await expect(page.locator('.proj-row')).toHaveCount(1);
    await expect(createdRow).toBeVisible();
    await expect(page.locator('#project-bulk-menu-trigger')).toBeDisabled();
    await page.getByRole('textbox', { name: 'Search by project or client' }).fill('no matching project');
    await expect(page.getByRole('heading', { name: 'No projects match your filters', exact: true })).toBeVisible();
    await page.getByRole('button', { name: 'Reset filters', exact: true }).click();
    await expect(page.locator('.proj-row')).toHaveCount(3);
    await expect(page.getByRole('button', { name: /^All tags/ })).toBeVisible();
    await readsFinished(page);
    console.log('PASS: real finalized budget reaches the authorized Projects endpoint and display');
    await page.goto(`${base}/timesheet/week/${process.env.HORAE_TEST_WEEK}`);
    await page.getByRole('button', { name: 'Add entry', exact: true }).click();
    const entry = page.getByRole('dialog', { name: /New time entry/ });
    await entry.getByRole('combobox', { name: 'Project', exact: true }).selectOption(created);
    await entry.getByRole('combobox', { name: 'Task', exact: true }).selectOption({ label: 'Development' });
    await entry.getByRole('textbox', { name: 'Duration', exact: true }).fill('1:00');
    await entry.getByPlaceholder('Notes (optional)').fill('Tagged project report check');
    await entry.getByRole('button', { name: 'Save entry', exact: true }).click();
    await expect(entry).not.toBeVisible();
    await readsFinished(page);
    await page.getByRole('link', { name: 'Reports', exact: true }).click();
    await page.locator('input[type="date"]').first().fill(process.env.HORAE_TEST_WEEK);
    await page.locator('input[type="date"]').last().fill(process.env.HORAE_TEST_WEEK);
    const reportTag = page.getByRole('combobox', { name: 'Project tag', exact: true });
    await expect(reportTag).toBeVisible();
    await readsFinished(page);
    let releaseReport;
    await page.route('**/api/report_time*', async route => {
      const response = await route.fetch();
      await new Promise(resolve => { releaseReport = resolve; });
      await route.fulfill({ response });
    });
    try {
      await reportTag.selectOption({ label: 'browser' });
      await expect.poll(() => !!releaseReport).toBe(true);
      await expect(page.getByRole('status')).toHaveText('Loading report…');
      await expect(page.locator('tbody tr')).toHaveCount(0);
    } finally {
      releaseReport?.();
    }
    const reportRows = page.locator('tbody tr:not(.report-total-row)');
    await expect(reportRows).toHaveCount(1);
    await expect(reportRows).toContainText('Recovered latest edit');
    await expect(reportRows.locator('td').nth(1)).toHaveText('1.00');
    await readsFinished(page);
    await page.unroute('**/api/report_time*');
    const tagId = await reportTag.inputValue();
    for (const format of ['CSV', 'XLSX']) {
      const href = await page.getByRole('link', { name: `Export ${format}`, exact: true }).getAttribute('href');
      assert.equal(new URL(href, base).searchParams.get('tag_id'), tagId);
      const response = await context.request.get(new URL(href, base).href);
      assert.equal(response.status(), 200);
      if (format === 'CSV') {
        const csv = await response.text();
        assert.equal(csv.trim().split('\n').length, 2, 'Only the tagged entry is exported');
        assert.match(csv, /Recovered latest edit/);
        assert.match(csv, /Tagged project report check/);
      } else {
        assert.equal((await response.body()).subarray(0, 2).toString(), 'PK');
      }
    }
    await page.getByRole('button', { name: 'Detailed time', exact: true }).click();
    await expect(page.locator('tbody tr')).toHaveCount(1);
    await expect(page.locator('tbody tr')).toContainText('Tagged project report check');
    await page.getByRole('button', { name: 'Time', exact: true }).click();
    await page.route('**/api/report_time*', route => route.abort());
    await reportTag.selectOption('');
    await expect(page.getByRole('alert')).toContainText('Could not load report');
    await expect(page.locator('tbody tr')).toHaveCount(0);
    await readsFinished(page);
    await page.unroute('**/api/report_time*');
    await page.getByRole('button', { name: 'Retry report', exact: true }).click();
    await expect(page.locator('tbody tr')).not.toHaveCount(0);
    await readsFinished(page);
    console.log('PASS: a new tagged project reaches reports and both exports; pending/error filters never show stale rows');
    await page.getByRole('link', { name: 'Invoices', exact: true }).click();
    await page.getByRole('button', { name: 'New Invoice', exact: true }).click();
    await page.getByLabel('Client', { exact: true }).selectOption({ label: 'New project browser client' });
    await page.getByLabel('Period from', { exact: true }).fill(process.env.HORAE_TEST_WEEK);
    await page.getByLabel('Period to', { exact: true }).fill(process.env.HORAE_TEST_WEEK);
    const generate = page.getByRole('button', { name: 'Generate draft', exact: true });
    const review = page.getByRole('button', { name: 'Review invoice', exact: true });
    await expect(generate).toBeDisabled();
    await review.click();
    await expect(generate).toBeEnabled();
    await expect(page.getByLabel('Days until payment is due', { exact: true })).toHaveValue('21');
    await expect(page.getByLabel('Tax (%)', { exact: true })).toHaveValue('21.00');
    await expect(page.getByLabel('Second tax name', { exact: true })).toHaveValue('Local tax');
    const charges = page.getByRole('table', { name: 'Estimated invoice charges (EUR)', exact: true });
    await expect(charges.locator('tbody tr')).toHaveCount(1);
    await expect(charges.locator('tfoot tr').last()).toHaveText(/TotalEUR 92.18/);
    await page.getByLabel('Tax (%)', { exact: true }).fill('NaN');
    await expect(generate).toBeDisabled();
    await expect(charges).toHaveCount(0);
    await review.click();
    await expect(page.getByText('Percentages must be between 0 and 100, with at most two decimal places.', { exact: true })).toBeVisible();
    await page.getByLabel('Tax (%)', { exact: true }).fill('21');
    await page.getByLabel('PO number', { exact: true }).fill('BROWSER-PO');
    await expect(generate).toBeDisabled();
    await page.getByRole('checkbox', { name: 'All projects for this client', exact: true }).click();
    await page.getByRole('checkbox', { name: 'Recovered latest edit', exact: true }).click();
    await review.click();
    await expect(generate).toBeEnabled();
    let releaseInvoice;
    let invoiceWrites = 0;
    await page.route('**/api/generate_invoice*', async route => {
      invoiceWrites++;
      const response = await route.fetch();
      await new Promise(resolve => { releaseInvoice = resolve; });
      await route.fulfill({ response });
    });
    const generated = page.waitForResponse(response => response.url().includes('/api/generate_invoice') && response.status() === 200);
    try {
      await generate.click();
      await expect.poll(() => !!releaseInvoice).toBe(true);
      await expect(generate).toBeDisabled();
      await expect(page.getByRole('button', { name: 'Cancel', exact: true })).toBeDisabled();
      await expect(page.getByLabel('PO number', { exact: true })).toBeDisabled();
    } finally {
      releaseInvoice?.();
    }
    const draft = (await (await generated).json()).invoice;
    assert.equal(invoiceWrites, 1);
    assert.deepEqual([draft.terms_days, draft.po_number, draft.tax1_bps, draft.tax2_bps, draft.total_cents], [21, 'BROWSER-PO', 2100, 150, 9218]);
    await expect(page).toHaveURL(`${base}/invoices/${draft.id}`);
    await expect(page.getByRole('heading', { name: `Invoice ${draft.number}`, exact: true })).toBeVisible();
    await readsFinished(page);
    await page.unroute('**/api/generate_invoice*');
    await page.getByRole('button', { name: 'Edit invoice values', exact: true }).click();
    await expect(page.getByRole('button', { name: 'Mark Sent', exact: true })).toBeDisabled();
    await page.getByLabel('Payment terms', { exact: true }).selectOption('45');
    await page.getByLabel('Discount (%)', { exact: true }).fill('12.50');
    await page.getByRole('button', { name: 'Remove second tax', exact: true }).click();
    await expect(page.getByLabel('Second tax name', { exact: true })).toHaveCount(0);
    await page.getByRole('button', { name: 'Add a second tax', exact: true }).click();
    await page.getByLabel('Second tax name', { exact: true }).fill('Municipal tax');
    await page.getByLabel('Second tax (%)', { exact: true }).fill('1.5');
    await page.route('**/api/update_invoice_defaults*', route => route.abort());
    await page.getByRole('button', { name: 'Save invoice values', exact: true }).click();
    await expect(page.locator('.alert-danger')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Save invoice values', exact: true })).toBeEnabled();
    await expect(page.getByLabel('Discount (%)', { exact: true })).toHaveValue('12.50');
    await expect(page.getByLabel('Second tax name', { exact: true })).toHaveValue('Municipal tax');
    await expect(page.getByRole('button', { name: 'Mark Sent', exact: true })).toBeDisabled();
    await readsFinished(page);
    await page.unroute('**/api/update_invoice_defaults*');
    const updated = page.waitForResponse(response => response.url().includes('/api/update_invoice_defaults') && response.status() === 200);
    await page.getByRole('button', { name: 'Save invoice values', exact: true }).click();
    const edited = await (await updated).json();
    assert.deepEqual([edited.terms_days, edited.discount_bps, edited.subtotal_cents, edited.discount_cents, edited.tax1_cents, edited.tax2_cents, edited.total_cents], [45, 1250, 7525, 941, 1383, 99, 8066]);
    assert.equal(edited.tax2_name, 'Municipal tax');
    await expect(page.locator('tfoot tr').last()).toHaveText(/TotalEUR 80.66/);
    await readsFinished(page);
    await page.reload();
    await expect(page.getByText('45 days', { exact: true })).toBeVisible();
    await expect(page.getByText('BROWSER-PO', { exact: true })).toBeVisible();
    await expect(page.getByText('Municipal tax (1.50%)', { exact: true })).toBeVisible();
    await expect(page.locator('tfoot tr').last()).toHaveText(/TotalEUR 80.66/);
    await page.getByRole('button', { name: 'Mark Sent', exact: true }).click();
    await expect(page.getByRole('button', { name: 'Mark Paid', exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Edit invoice values', exact: true })).toHaveCount(0);
    await readsFinished(page);
    console.log('PASS: project invoice defaults prefill real estimates, stale reviews cannot generate, and editable draft values persist independently');
    await page.goto(`${base}/projects/new`);
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('');
    await expect(draftStatus).toHaveText('No draft saved yet');
    console.log('PASS: lost creation response is idempotent and clears the completed draft');

    await screen.getByLabel('Project name', { exact: true }).fill('Two tab draft');
    await saved();
    const second = await context.newPage();
    try {
      await second.goto(`${base}/projects/new`);
      await expect(second.getByLabel('Project name', { exact: true })).toHaveValue('Two tab draft');
      await screen.getByLabel('Project name', { exact: true }).fill('First tab acknowledged');
      await saved();
      await second.getByLabel('Project name', { exact: true }).fill('Stale tab edit');
      await expect(second.locator('.np-page').getByRole('alert')).toBeVisible();
      await expect(second.locator('.np-page header').getByRole('status')).toHaveText('Changes need attention');
      await second.getByRole('button', { name: 'Reload saved draft (lose local edits)', exact: true }).click();
      await expect(second.getByLabel('Project name', { exact: true })).toHaveValue('First tab acknowledged');
    } finally {
      await readsFinished(second);
      await second.close();
    }
    await screen.getByRole('button', { name: 'Discard draft', exact: true }).click();
    const discard = page.getByRole('dialog', { name: 'Discard this draft?', exact: true });
    await discard.getByRole('button', { name: 'Keep editing', exact: true }).click();
    await expect(discard).not.toBeVisible();
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('First tab acknowledged');
    await screen.getByRole('button', { name: 'Discard draft', exact: true }).click();
    await discard.getByRole('button', { name: 'Discard draft', exact: true }).click();
    await expect(page).toHaveURL(`${base}/projects`);
    await expect(page.getByRole('heading', { name: 'Projects', exact: true })).toBeVisible();
    await readsFinished(page);
    await page.goto(`${base}/projects/new`);
    await expect(screen.getByLabel('Project name', { exact: true })).toHaveValue('');
    await expect(draftStatus).toHaveText('No draft saved yet');
    console.log('PASS: stale tabs cannot overwrite saved input; explicit discard removes only the draft');
    assert.deepEqual(errors, []);
  } catch (error) {
    console.error({ url: page.url(), errors, pending: [...requests.get(page)].map(request => new URL(request.url()).pathname),
      resources: [...resources.get(page)].map(request => {
        const url = new URL(request.url());
        return { type: request.resourceType(), host: url.host, path: url.pathname };
      }), page: await page.locator('body').ariaSnapshot() });
    throw error;
  } finally {
    await browser.close();
  }
})().catch(error => { console.error(error); process.exitCode = 1; });
