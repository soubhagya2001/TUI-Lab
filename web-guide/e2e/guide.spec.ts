import { expect, test } from '@playwright/test'

const CHAPTERS = [
  { hash: '#/getting-started', heading: 'Getting started' },
  { hash: '#/writing-tests', heading: 'Writing tests' },
  { hash: '#/assertions-snapshots', heading: 'Assertions & snapshots' },
  { hash: '#/recorder', heading: 'Recorder' },
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

  test('search finds chapters', async ({ page }) => {
    await page.goto('./')
    await page.getByRole('textbox', { name: 'Search the guide' }).fill('snapshot')
    await expect(page.getByRole('link', { name: /Assertions & snapshots/ })).toBeVisible()
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

  test('no dead external links', async ({ page, request }) => {
    await page.goto('./')
    const hrefs = await page.locator('a[href]').evaluateAll((links) =>
      [...new Set(links.map((link) => link.getAttribute('href') ?? ''))].filter(Boolean),
    )
    const external = hrefs.filter((href) => href.startsWith('http'))
    expect(external.length).toBeGreaterThan(0)
    for (const href of external) {
      const response = await request.get(href, { timeout: 15000 })
      expect(response.ok(), `Dead external link: ${href} returned ${response.status()}`).toBeTruthy()
    }
  })
})
