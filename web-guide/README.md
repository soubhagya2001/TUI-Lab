# TUI Lab Developer Guide (`web-guide/`)

React 19 + Vite + MDX documentation site for TUI Lab. Deployed to GitHub
Pages via `.github/workflows/web-guide.yml`.

## Develop

```bash
npm ci
npm run dev      # local preview with hot reload
npm run build    # typecheck + static build to dist/
```

## Test

```bash
npm run test:e2e          # Playwright (chromium) against `vite preview`
npm run test:e2e:headed   # headed mode for debugging
npm run test:e2e:report   # open the last HTML report
```

Conventions: shadcn components first (see `.agents/skills/shadcn`),
`flex` + `gap-*` (never `space-x/y`), semantic color tokens only.
Content lives in `src/content/*.mdx` (one file per route); shared step and
terminal components in `src/components/`. Hash routing (`HashRouter`) —
project Pages serves `index.html` only, so no browser-history fallback.

## Deploy

Push to `main` touching `web-guide/**` (or dispatch manually): the workflow
builds, runs Playwright, and deploys `dist/` via `actions/deploy-pages`.
PRs get a build-only check (no deploy).
