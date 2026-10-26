import { test, expect, Page } from '@playwright/test';
import fs from 'node:fs/promises';

const server = 'http://127.0.0.1:3100';
const token = (name: string) => `${name}-browser-test-token-00000000000000000000`;
const status = (page: Page) => page.getByTestId('sharing-status');

async function connect(page: Page, name: string, id = '') {
  await page.goto('/');
  await page.getByText('Project sharing', { exact: true }).click();
  await page.getByLabel('Server URL', { exact: true }).fill(server);
  await page.getByLabel('Bearer token', { exact: true }).fill(token(name));
  await page.getByLabel('Shared project ID', { exact: true }).fill(id);
  page.on('dialog', dialog => dialog.accept());
}
async function download(page: Page) {
  await page.getByRole('button', { name: 'Download project', exact: true }).click();
  await expect(status(page)).toContainText('Download ready');
}
async function save(page: Page, path: string) {
  const download = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Save project', exact: true }).click();
  await (await download).saveAs(path);
  return fs.readFile(path, 'utf8');
}

test('two browser clients exchange the complete draft, resolve stale writes, and enforce access', async ({ browser, request }, info) => {
  test.setTimeout(90_000);
  const first = await browser.newContext();
  const second = await browser.newContext();
  const alice = await first.newPage();
  const bob = await second.newPage();
  const errors: string[] = [];
  alice.on('pageerror', e => errors.push(e.message));
  bob.on('pageerror', e => errors.push(e.message));
  await connect(alice, 'alice');
  await alice.getByLabel('Open project', { exact: true }).setInputFiles('fixtures/three-rooms.pace-draft.json');
  await expect(alice.locator('.project-title')).toContainText('The Garden Key');
  await alice.getByRole('button', { name: 'Project settings', exact: true }).click();
  await expect(alice.getByLabel('Project title', { exact: true })).toHaveValue('The Garden Key');
  await alice.getByRole('button', { name: 'Upload as new project', exact: true }).click();
  await expect(status(alice)).toContainText('Project uploaded');
  const id = await alice.getByLabel('Shared project ID', { exact: true }).inputValue();
  expect(id).toMatch(/^[a-f0-9-]{36}$/);
  await alice.getByLabel('Member ID', { exact: true }).fill('bob');
  await alice.getByLabel('Member access', { exact: true }).selectOption('writer');
  await alice.getByRole('button', { name: 'Grant access', exact: true }).click();
  await expect(status(alice)).toContainText('Granted writer access to bob');

  await connect(bob, 'bob', id);
  await download(bob);
  await expect(bob.getByLabel('Project title', { exact: true })).toHaveValue('Untitled adventure');
  await bob.getByRole('button', { name: 'Open downloaded project', exact: true }).click();
  await bob.getByRole('button', { name: 'Project settings', exact: true }).click();
  const fixture = JSON.parse(await fs.readFile('fixtures/three-rooms.pace-draft.json', 'utf8'));
  const saved = await save(bob, info.outputPath('downloaded.pace-draft.json'));
  expect(JSON.parse(saved)).toEqual(fixture);
  expect(saved).not.toContain(token('bob'));
  expect(saved).not.toContain(id);
  await expect(bob.getByText('Validation · 0', { exact: true })).toBeVisible();

  await bob.getByLabel('Project title', { exact: true }).fill('Bob changed the title');
  await bob.getByRole('button', { name: 'Upload changes', exact: true }).click();
  await expect(status(bob)).toContainText('Changes uploaded');
  await alice.getByLabel('Project title', { exact: true }).fill('Alice kept her edits');
  await alice.getByRole('button', { name: 'Upload changes', exact: true }).click();
  await expect(status(alice)).toContainText('Conflict: the server has revision 2');
  await expect(alice.getByLabel('Project title', { exact: true })).toHaveValue('Alice kept her edits');
  const headers = { Authorization: `Bearer ${token('alice')}` };
  expect((await (await request.get(`${server}/v1/projects/${id}`, { headers })).json()).title).toBe('Bob changed the title');
  await download(alice);
  await expect(alice.getByLabel('Project title', { exact: true })).toHaveValue('Alice kept her edits');
  await alice.getByRole('button', { name: 'Keep local edits against downloaded revision', exact: true }).click();
  await alice.getByRole('button', { name: 'Upload changes', exact: true }).click();
  await expect(status(alice)).toContainText('Changes uploaded');
  const latest = await request.get(`${server}/v1/projects/${id}`, { headers });
  expect(latest.headers().etag).toBe('"3"');
  expect(await latest.json()).toEqual({ ...fixture, title: 'Alice kept her edits' });

  await download(bob);
  await bob.getByRole('button', { name: 'Open downloaded project', exact: true }).click();
  await bob.getByRole('button', { name: 'Project settings', exact: true }).click();
  expect(JSON.parse(await save(bob, info.outputPath('after-conflict.pace-draft.json')))).toEqual({ ...fixture, title: 'Alice kept her edits' });
  await bob.getByLabel('Bearer token', { exact: true }).fill('invalid-token-00000000000000000000000000');
  await bob.getByRole('button', { name: 'Download project', exact: true }).click();
  await expect(status(bob)).toContainText('Authentication failed');
  await expect(bob.getByLabel('Project title', { exact: true })).toHaveValue('Alice kept her edits');
  await bob.getByLabel('Bearer token', { exact: true }).fill(token('eve'));
  await bob.getByRole('button', { name: 'Download project', exact: true }).click();
  await expect(status(bob)).toContainText('Project not found or access has not been granted');
  await bob.getByLabel('Bearer token', { exact: true }).fill(token('bob'));
  await alice.getByRole('button', { name: 'Revoke access', exact: true }).click();
  await expect(status(alice)).toContainText('Revoked access for bob');
  await bob.getByRole('button', { name: 'Download project', exact: true }).click();
  await expect(status(bob)).toContainText('Project not found or access has not been granted');
  const storage = await bob.evaluate(() => JSON.stringify({ local: { ...localStorage }, session: { ...sessionStorage } }));
  expect(storage).not.toContain(token('bob'));
  await alice.screenshot({ path: info.outputPath('project-sharing.png'), fullPage: true });
  await bob.reload();
  await bob.getByText('Project sharing', { exact: true }).click();
  await expect(bob.getByLabel('Bearer token', { exact: true })).toHaveValue('');
  expect(errors).toEqual([]);
  await first.close(); await second.close();
});

test('failed and late downloads cannot replace local work', async ({ page, request }) => {
  const fixture = JSON.parse(await fs.readFile('fixtures/three-rooms.pace-draft.json', 'utf8'));
  const response = await request.post(`${server}/v1/projects`, { headers: { Authorization: `Bearer ${token('alice')}` }, data: fixture });
  expect(response.status()).toBe(201);
  const id = (await response.json()).id;
  await connect(page, 'alice', id);
  await page.getByLabel('Project title', { exact: true }).fill('Keep my local work');
  const url = `${server}/v1/projects/${id}`;
  await page.route(url, route => route.fulfill({ status: 200, headers: {
    ETag: '"1"', 'Content-Type': 'application/json',
    'Access-Control-Allow-Origin': 'http://127.0.0.1:8080', 'Access-Control-Expose-Headers': 'ETag',
  }, body: '{"version":999}' }));
  await page.getByRole('button', { name: 'Download project', exact: true }).click();
  await expect(status(page)).toContainText('not a supported editor draft');
  await expect(page.getByLabel('Project title', { exact: true })).toHaveValue('Keep my local work');
  await page.unroute(url);
  let release!: () => void;
  const gate = new Promise<void>(resolve => { release = resolve; });
  await page.route(url, async route => { await gate; await route.continue(); });
  await page.getByRole('button', { name: 'Download project', exact: true }).click();
  await expect(status(page)).toContainText('Downloading project');
  await page.getByRole('button', { name: 'New project', exact: true }).click();
  release();
  await expect(status(page)).toContainText('different document was opened');
  await expect(page.getByLabel('Project title', { exact: true })).toHaveValue('Untitled adventure');
  await expect(page.getByRole('button', { name: 'Upload changes', exact: true })).toBeDisabled();
});
