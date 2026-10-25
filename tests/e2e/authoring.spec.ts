import { test, expect, Page } from '@playwright/test';
import fs from 'node:fs/promises';

async function room(page: Page, name: string, image: string) {
  await page.getByRole('button', { name: 'Add room', exact: true }).click();
  await page.getByLabel('Room name', { exact: true }).fill(name);
  await page.getByLabel('Import background', { exact: true }).setInputFiles(`fixtures/backgrounds/${image}.png`);
  await expect(page.getByRole('img', { name: `${name} background` })).toBeVisible();
}
async function hotspot(page: Page, room: string, name: string, bounds: number[], action: string, target: string) {
  await page.getByRole('button', { name: room, exact: true }).click();
  await page.getByRole('button', { name: 'Add hotspot', exact: true }).click();
  await page.getByLabel('Hotspot name', { exact: true }).fill(name);
  for (const [i, field] of ['X', 'Y', 'Width', 'Height'].entries())
    await page.getByLabel(`${field} (%)`, { exact: true }).fill(String(bounds[i]));
  await page.getByRole('combobox', { name: 'Action', exact: true }).selectOption(action);
  if (action === 'look') await page.getByLabel('Inspection text', { exact: true }).fill(target);
  else await page.getByRole('combobox', { name: { go_to: 'Destination room', take: 'Item to pick up', talk: 'Dialogue to start' }[action]!, exact: true }).selectOption({ label: target });
}

test('authors the three-room adventure entirely through controls and rehearses the story', async ({ page }, testInfo) => {
  test.setTimeout(90_000);
  const errors: string[] = [];
  page.on('pageerror', e => errors.push(e.message));
  page.on('dialog', d => d.accept());
  await page.goto('/');
  await page.getByLabel('Project title', { exact: true }).fill('The Garden Key');
  await room(page, 'Foyer', 'foyer');
  await room(page, 'Study', 'study');
  await room(page, 'Garden', 'garden');
  for (const [name, description] of [['Brass key', 'Opens the study door.'], ['Garden pass', 'A gift from the gardener.']]) {
    await page.getByRole('button', { name: 'Add item', exact: true }).click();
    await page.getByLabel('Item name', { exact: true }).fill(name);
    await page.getByLabel('Item description', { exact: true }).fill(description);
  }
  await page.getByRole('button', { name: 'Add dialogue', exact: true }).click();
  await page.getByLabel('Dialogue name', { exact: true }).fill('Farewell');
  await page.getByLabel('Speaker', { exact: true }).fill('Gardener');
  await page.getByLabel('Dialogue text', { exact: true }).fill('The garden is waiting for you.');
  await page.getByRole('button', { name: 'Add dialogue', exact: true }).click();
  await page.getByLabel('Dialogue name', { exact: true }).fill('Meet the gardener');
  await page.getByLabel('Speaker', { exact: true }).fill('Gardener');
  await page.getByLabel('Dialogue text', { exact: true }).fill('Would you like to visit the garden?');
  await page.getByRole('button', { name: 'Add choice', exact: true }).click();
  await page.getByLabel('Choice text', { exact: true }).fill('May I enter?');
  await page.getByRole('combobox', { name: 'Next dialogue', exact: true }).selectOption({ label: 'Farewell' });
  await page.getByRole('combobox', { name: 'Give item', exact: true }).selectOption({ label: 'Garden pass' });
  await page.getByRole('button', { name: 'Add choice', exact: true }).click();
  await page.getByLabel('Choice text', { exact: true }).nth(1).fill('Not yet.');
  await hotspot(page, 'Foyer', 'Brass key on table', [44, 60, 16, 15], 'take', 'Brass key');
  await hotspot(page, 'Foyer', 'Study door', [72, 20, 20, 48], 'go_to', 'Study');
  await page.getByRole('combobox', { name: 'Requires item', exact: true }).selectOption({ label: 'Brass key' });
  await hotspot(page, 'Study', 'Return to foyer', [8, 20, 18, 48], 'go_to', 'Foyer');
  await hotspot(page, 'Study', 'Talk to gardener', [40, 55, 24, 18], 'talk', 'Meet the gardener');
  await hotspot(page, 'Study', 'Garden gate', [72, 20, 20, 48], 'go_to', 'Garden');
  await page.getByRole('combobox', { name: 'Requires item', exact: true }).selectOption({ label: 'Garden pass' });
  await hotspot(page, 'Garden', 'Return to study', [8, 20, 18, 48], 'go_to', 'Study');
  await hotspot(page, 'Garden', 'Statue', [42, 40, 18, 30], 'look', 'The garden remembers every visitor.');
  await expect(page.getByText('Validation · 0', { exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Core preview', exact: true })).toBeDisabled();
  await expect(page.getByRole('button', { name: 'Export game', exact: true })).toBeDisabled();
  const downloadPromise = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Save project', exact: true }).click();
  const download = await downloadPromise;
  const file = testInfo.outputPath('three-rooms.pace-draft.json');
  await download.saveAs(file);
  const saved = JSON.parse(await fs.readFile(file, 'utf8'));
  expect(saved.rooms.map((r: any) => r.name)).toEqual(['Foyer', 'Study', 'Garden']);
  expect(saved.rooms.flatMap((r: any) => r.hotspots)).toHaveLength(7);
  expect(saved.items).toHaveLength(2); expect(saved.dialogues).toHaveLength(2);
  await page.getByRole('button', { name: 'Foyer', exact: true }).click();
  await page.getByRole('button', { name: 'Select hotspot Study door', exact: true }).click();
  await page.screenshot({ path: testInfo.outputPath('01-room-editor.png'), fullPage: true });
  if (process.env.UPDATE_WALKTHROUGH) {
    await fs.copyFile(file, 'fixtures/three-rooms.pace-draft.json');
    await page.screenshot({ path: 'docs/images/01-room-editor.png', fullPage: true });
  }
  await page.getByRole('button', { name: 'Meet the gardener', exact: true }).click();
  await expect(page.getByRole('combobox', { name: 'Starting room', exact: true }).locator('option:checked')).toHaveText('Foyer');
  await expect(page.getByRole('combobox', { name: 'Next dialogue', exact: true }).first().locator('option:checked')).toHaveText('Farewell');
  await expect(page.getByRole('combobox', { name: 'Give item', exact: true }).first().locator('option:checked')).toHaveText('Garden pass');
  await page.screenshot({ path: testInfo.outputPath('02-dialogue-editor.png'), fullPage: true });
  if (process.env.UPDATE_WALKTHROUGH) await page.screenshot({ path: 'docs/images/02-dialogue-editor.png', fullPage: true });
  await page.getByRole('button', { name: 'Rehearse draft', exact: true }).click();
  const preview = page.getByRole('dialog', { name: 'Draft rehearsal' });
  await preview.getByRole('button', { name: 'Interact with Study door', exact: true }).click();
  await expect(preview.getByRole('status')).toHaveText('You need Brass key first.');
  await preview.getByRole('button', { name: 'Interact with Brass key on table', exact: true }).click();
  await preview.getByRole('button', { name: 'Interact with Study door', exact: true }).click();
  await expect(preview.getByRole('heading', { name: 'Study', exact: true })).toBeVisible();
  await preview.getByRole('button', { name: 'Interact with Garden gate', exact: true }).click();
  await expect(preview.getByRole('status')).toHaveText('You need Garden pass first.');
  await preview.getByRole('button', { name: 'Interact with Talk to gardener', exact: true }).click();
  await preview.getByRole('button', { name: 'Not yet.', exact: true }).click();
  await expect(preview.getByRole('status')).toHaveText('The conversation ends.');
  await preview.getByRole('button', { name: 'Interact with Talk to gardener', exact: true }).click();
  await preview.getByRole('button', { name: 'May I enter?', exact: true }).click();
  await expect(preview.getByText('The garden is waiting for you.', { exact: true })).toBeVisible();
  await preview.getByRole('button', { name: 'Continue', exact: true }).click();
  await preview.getByRole('button', { name: 'Interact with Garden gate', exact: true }).click();
  await expect(preview.getByRole('heading', { name: 'Garden', exact: true })).toBeVisible();
  await preview.getByRole('button', { name: 'Interact with Statue', exact: true }).click();
  await expect(preview.getByRole('status')).toHaveText('The garden remembers every visitor.');
  if (process.env.UPDATE_WALKTHROUGH) await page.screenshot({ path: 'docs/images/03-draft-rehearsal.png' });
  await preview.getByRole('button', { name: 'Interact with Return to study', exact: true }).click();
  await preview.getByRole('button', { name: 'Interact with Return to foyer', exact: true }).click();
  await preview.getByRole('button', { name: 'Restart', exact: true }).click();
  await expect(preview.getByText('Nothing collected yet', { exact: true })).toBeVisible();
  await preview.getByRole('button', { name: 'Close rehearsal', exact: true }).click();
  await expect(preview).toHaveCount(0);
  await page.getByRole('button', { name: 'Rehearse draft', exact: true }).click();
  // The native modal keeps keyboard focus away from editing controls.
  for (let i = 0; i < 7; i++) {
    await page.keyboard.press('Tab');
    expect(await page.evaluate(() => document.activeElement === document.body || !!document.activeElement?.closest('#draft-rehearsal'))).toBe(true);
  }
  await page.keyboard.press('Escape');
  await expect(preview).toHaveCount(0);
  // Rehearsal must not dirty or change the authored data.
  const afterPromise = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Save project', exact: true }).click();
  await (await afterPromise).saveAs(testInfo.outputPath('after-rehearsal.json'));
  expect(JSON.parse(await fs.readFile(testInfo.outputPath('after-rehearsal.json'), 'utf8'))).toEqual(saved);
  await page.getByRole('button', { name: 'New project', exact: true }).click();
  await page.getByLabel('Open project', { exact: true }).setInputFiles(file);
  await expect(page.getByText('Validation · 0', { exact: true })).toBeVisible();
  const reopenedPromise = page.waitForEvent('download');
  await page.getByRole('button', { name: 'Save project', exact: true }).click();
  await (await reopenedPromise).saveAs(testInfo.outputPath('reopened.json'));
  expect(JSON.parse(await fs.readFile(testInfo.outputPath('reopened.json'), 'utf8'))).toEqual(saved);
  expect(errors).toEqual([]);
});
