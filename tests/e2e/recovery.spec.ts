import { test, expect } from '@playwright/test';
import fs from 'node:fs/promises';

test('quota failure preserves edits and offers a portable file; malformed open is non-destructive', async ({ page }, testInfo) => {
  page.on('dialog', d => d.accept());
  await page.addInitScript(() => {
    Storage.prototype.setItem = () => { throw new DOMException('Quota exceeded', 'QuotaExceededError'); };
  });
  await page.goto('/');
  await page.getByLabel('Project title', { exact: true }).fill('Keep this draft');
  const downloadPromise = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Save project', exact: true }).click();
  const file = testInfo.outputPath('quota-draft.json');
  await (await downloadPromise).saveAs(file);
  expect(JSON.parse(await fs.readFile(file, 'utf8')).title).toBe('Keep this draft');
  await expect(page.getByRole('status')).toContainText('Browser recovery is full or unavailable');
  await expect(page.locator('.project-title')).toContainText('•');
  await page.getByLabel('Open project', { exact: true }).setInputFiles({ name: 'broken.json', mimeType: 'application/json', buffer: Buffer.from('{broken') });
  await expect(page.getByRole('status')).toContainText('Invalid project JSON');
  await expect(page.getByLabel('Project title', { exact: true })).toHaveValue('Keep this draft');
});

test('cancelled destructive action retains the current draft', async ({ page }) => {
  page.on('dialog', d => d.dismiss());
  await page.goto('/');
  await page.getByLabel('Project title', { exact: true }).fill('Unsaved story');
  await page.getByRole('button', { name: 'New project', exact: true }).click();
  await expect(page.getByLabel('Project title', { exact: true })).toHaveValue('Unsaved story');
});
