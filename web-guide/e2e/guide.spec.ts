import { expect, test } from '@playwright/test'

const CHAPTERS = [
  { hash: '#/getting-started', heading: 'Getting started' },
  { hash: '#/writing-tests', heading: 'Writing tests' },
  { hash: '#/assertions-snapshots', heading: 'Assertions & snapshots' },
  { hash: '#/recorder', heading: 'Recorder' },
  { hash: '#/recipes', heading: 'Recipes' },
  { hash: '#/reference', heading: 'Architecture & reference' },
  { hash: '#/cli-reference', heading: 'CLI reference' },
  { hash: '#/mcp-agents', heading: 'MCP & AI agents' },
  { hash: '#/sdks', heading: 'SDKs' },
  { hash: '#/troubleshooting', heading: 'Troubleshooting' },
]

test.describe('Guide site', () => {
  test('home renders hero with animation terminal', async ({ page }) => {
    await page.goto('./')
    await expect(page.getByRole('heading', { name: /Playwright for Terminal Applications/ })).toBeVisible()
    await expect(page.getByRole('link', { name: 'Get started' })).toBeVisible()
    await expect(page.getByRole('link', { name: 'How it works' })).toBeVisible()
    // Animated terminal finishes its loop on the result line.
    await expect(page.getByText('2 passed, 0 failed')).toBeVisible({ timeout: 20000 })
    await expect(page.getByRole('heading', { name: 'How it works' })).toBeVisible()
  })

  test('sidebar navigates to every chapter', async ({ page }) => {
    await page.goto('./')
    for (const chapter of CHAPTERS) {
      await page.getByRole('navigation', { name: 'Guide chapters' }).getByRole('link', { name: chapter.heading }).click()
      await expect(page).toHaveURL(new RegExp(`${chapter.hash}$`))
      await expect(page.getByRole('heading', { name: chapter.heading, exact: true }).first()).toBeVisible()
    }
  })

  test('theme toggle switches dark and light', async ({ page }) => {
    await page.goto('./')
    const html = page.locator('html')
    await expect(html).toHaveClass(/dark/)
    await page.getByRole('button', { name: 'Switch to light theme' }).click()
    await expect(html).not.toHaveClass(/dark/)
    await expect(page.getByRole('button', { name: 'Switch to dark theme' })).toBeVisible()
    await page.getByRole('button', { name: 'Switch to dark theme' }).click()
    await expect(html).toHaveClass(/dark/)
  })

  test('search shows chapter suggestions while typing', async ({ page }) => {
    await page.goto('./')
    await page.getByRole('combobox', { name: 'Search the guide' }).pressSequentially('snapshot')
    const dropdown = page.getByTestId('search-results')
    await expect(dropdown.getByRole('option').first()).toBeVisible()
    await expect(dropdown.getByRole('option', { name: /Assertions & snapshots/ })).toBeVisible()
  })

  test('prev/next walk the whole guide', async ({ page }) => {
    await page.goto('./#/getting-started')
    for (const chapter of CHAPTERS.slice(1)) {
      await page.getByRole('link', { name: new RegExp(`${chapter.heading} →`) }).click()
      await expect(page.getByRole('heading', { name: chapter.heading, exact: true }).first()).toBeVisible()
    }
  })

  test('no console errors on any route', async ({ page }) => {    const errors: string[] = []
    page.on('pageerror', (error) => errors.push(error.message))
    page.on('console', (message) => {
      if (message.type() === 'error') errors.push(message.text())
    })
    await page.goto('./')
    for (const chapter of CHAPTERS) {
      await page.goto(`./${chapter.hash}`)
      await expect(page.getByRole('heading', { name: chapter.heading, exact: true }).first()).toBeVisible()
    }
    expect(errors).toEqual([])
  })

  test('chapter cards have vertical rhythm', async ({ page }) => {
    await page.goto('./#/getting-started')
    const cards = page.locator('.chapter-body > *')
    const count = await cards.count()
    expect(count).toBeGreaterThan(2)
    let previousBottom = -Infinity
    for (let i = 0; i < count; i++) {
      const box = await cards.nth(i).boundingBox()
      if (!box || box.height === 0) continue
      expect(box.y, `card ${i} overlaps the previous block`).toBeGreaterThan(previousBottom + 8)
      previousBottom = box.y + box.height
    }
  })

  test('snippet copy buttons copy code to clipboard', async ({ page, context }) => {
    await context.grantPermissions(['clipboard-read', 'clipboard-write'])
    await page.goto('./#/getting-started')
    await page.getByRole('button', { name: 'Copy code' }).first().click()
    const clipped = await page.evaluate(() => navigator.clipboard.readText())
    expect(clipped).toContain('pip install tui-lab')
  })

  test('footer shows contact links', async ({ page }) => {
    await page.goto('./')
    const footer = page.locator('footer')
    await expect(footer.getByText('Contact us')).toBeVisible()
    await expect(footer.getByRole('link', { name: 'Email' })).toHaveAttribute('href', 'mailto:soubhagyaprusty36@gmail.com')
    await expect(footer.getByRole('link', { name: 'LinkedIn' })).toHaveAttribute(
      'href',
      'https://linkedin.com/in/soubhagya-prusty-5424811b6',
    )
    await expect(footer.getByRole('link', { name: 'GitHub' })).toHaveAttribute(
      'href',
      'https://github.com/soubhagya2001',
    )
  })

  test('external links are well-formed (no live GETs)', async ({ page }) => {
    // D4: link checking must not depend on the network — assert shape.
    // The footer test already pins the exact contact URLs.
    await page.goto('./')
    const hrefs = await page.locator('a[href]').evaluateAll((links) =>
      [...new Set(links.map((link) => link.getAttribute('href') ?? ''))].filter(Boolean),
    )
    const external = hrefs.filter((href) => href.startsWith('http'))
    expect(external.length).toBeGreaterThan(0)
    for (const href of external) {
      expect(href, `malformed URL: ${href}`).toMatch(/^https:\/\/[A-Za-z0-9.-]+\//)
    }
  })

  test('search keyboard flow and empty state', async ({ page }) => {
    await page.goto('./')
    const box = page.getByRole('combobox', { name: 'Search the guide' })
    await box.pressSequentially('snapshot')
    const dropdown = page.getByTestId('search-results')
    await expect(dropdown.getByRole('option').first()).toBeVisible()
    await page.keyboard.press('ArrowDown')
    await page.keyboard.press('Enter')
    await expect(page).toHaveURL(/#\/[a-z-]+$/)
    // No-match query shows the empty state instead of suggestions.
    await page.goto('./')
    await page.getByRole('combobox', { name: 'Search the guide' }).pressSequentially('zzz-no-such-topic')
    await expect(page.getByTestId('search-results')).toBeVisible()
    await expect(page.getByTestId('search-results')).toContainText('No matches')
  })

  test('sidebar groups chapters into sections', async ({ page }) => {
    await page.goto('./')
    const nav = page.getByRole('navigation', { name: 'Guide chapters' })
    for (const section of ['Start', 'Write', 'Reference', 'Help']) {
      await expect(nav.getByText(section, { exact: true })).toBeVisible()
    }
  })

  test('unknown routes show the 404 chapter', async ({ page }) => {
    await page.goto('./#/no-such-page')
    await expect(page.getByText('That page isn’t in this guide')).toBeVisible()
    await expect(page.getByRole('link', { name: 'Back to Home' })).toBeVisible()
  })
})
