# 15 — Web Guide Plan (`web-guide/` developer docs site)

> **Status:** built and tested (Windows-verified). App at repo-root `web-guide/`,
> Vite + React 19 + MDX, terminal-green accent, dark default + light toggle.
> Playwright e2e (chromium, 7 tests) green; `web-guide.yml` builds, tests,
> and deploys to GitHub Pages.

## 15.1 What this is

A React-based documentation website for TUI Lab — a step-by-step developer
guide to installing, writing, running, and debugging terminal-app tests.
It distills `docs/01`–`docs/14` into a user-friendly journey; the `docs/`
files remain the design source of truth and the site links back to them.

## 15.2 Stack (DECIDED)

*   **Vite 6 + React 19 + TypeScript** — static build, ideal for GitHub Pages.
*   **React Router with hash routing** (`HashRouter`) — project Pages serves
    `index.html` only; browser-history fallback does not exist there.
*   **MDX for chapter content** — via `@mdx-js/rollup` + `remark-gfm` +
    `rehype-highlight` (highlight.js; `rehype-shiki` rejected — its
    `oniguruma` native dep needs node-gyp/Python unavailable here).
*   **Tailwind CSS + shadcn/ui** — all UI from reusable shadcn components
    (see §15.4); semantic OKLCH tokens only, dark default + light toggle.
*   **Theme:** terminal-green accent (phosphor green on near-black dark,
    deep green on paper light); Inter + JetBrains Mono; see §15.10.
*   **Animation:** `motion` (Framer Motion successor) for scroll reveals,
    page transitions, and the animated hero terminal (§15.11); CSS
    transitions for the rest. No heavy 3D. Honors `prefers-reduced-motion`.
*   **Search:** client-side (e.g. `flexsearch` index built at build time).
*   **Node 20**, `npm ci` reproducible builds.

## 15.3 Local skill (installed)

*   **`.agents/skills/shadcn`** (official `shadcn/ui@shadcn`, 271.7K installs,
    added project-local via `npx skills add shadcn/ui@shadcn`) — governs all
    component work in `web-guide/`. Key rules baked in from the skill:
    use existing components first (`npx shadcn@latest search`), compose don't
    reinvent, built-in variants before custom styles, semantic colors only
    (`bg-primary`, `text-muted-foreground` — never raw `bg-blue-500`),
    `flex` + `gap-*` (never `space-x/y-*`), `cn()` for conditional classes,
    full `Card` composition, `Alert` for callouts, `Empty` for empty states,
    `Separator`/`Skeleton`/`Badge` instead of custom markup.
*   Considered and deferred: `anthropics/skills/frontend-design`,
    `vercel-labs/agent-skills/vercel-react-best-practices` — revisit if the
    build needs design-system or React-pattern guidance beyond shadcn.

## 15.4 Reusable components

shadcn (added via `npx shadcn@latest add <name>`, owned by the site, themed
via CSS variables):

*   Layout/nav: `Sidebar`, `Sheet` (mobile nav), `Breadcrumb`, `Separator`,
    `ScrollArea`, `Tabs` (SDK Python/JS/Rust switcher).
*   Content: `Card` (+Header/Title/Description/Content/Footer), `Alert`
    (callouts: DECIDED / gotcha / Windows-note), `Badge` (phase tags,
    `v1`/`v2` labels), `Accordion` (FAQ), `Table`, `Empty`, `Skeleton`.
*   Code: `CodeBlock` wrapper (copy button + language label) around fenced
    MDX code; `Terminal` custom component for the animated hero + step I/O.
*   Feedback/motion: `Tooltip`, `Dialog` (image zoom), page-transition +
    `whileInView` reveals via `motion`.

Custom (only what shadcn lacks): `StepBlock` (number → explanation →
command → expected output), `Terminal` (typing animation), `CodeTabs`,
`Mermaid` (architecture diagram from `docs/02`), `FaqAccordion` (on top of
shadcn `Accordion`), `SdkTabs` (on top of shadcn `Tabs`).

## 15.5 What to show / how to show

A step-by-step user journey, not a spec dump. Every chapter: numbered steps,
each step = short explanation + copyable command/YAML + expected output in
`Terminal` styling. Windows/ConPTY callouts wherever `docs/12` Pitfalls
apply. Screenshots/GIFs of sample runs land later in `web-guide/public/`.

## 15.6 Pages (9 routes + source mapping)

| Route | Title | Sources |
|-------|-------|---------|
| `/` | Home — hero + 60-second quickstart | `docs/01`, `docs/14` |
| `/getting-started` | Install, `tuilab init`, first green `run` | `docs/07`, `docs/14` |
| `/writing-tests` | YAML DSL step reference + examples | `docs/05` |
| `/assertions-snapshots` | Taxonomy, masking, retry semantics | `docs/06` |
| `/recorder` | `record` workflow + smart waits | `docs/10` |
| `/cli-reference` | Commands, flags, `tuilab.yaml`, exit codes | `docs/07` |
| `/mcp-agents` | 9 MCP tools, Modes A/B, allowlist + cwd jail | `docs/08` |
| `/sdks` | Setup + sketches, tabbed Py/JS/Rust | `docs/09` |
| `/ci-troubleshooting` | CI example, JUnit/HTML reports, failure bundle, FAQ | `docs/11`, `docs/12` |

Shared layout: sidebar nav (shadcn `Sidebar`), top search, dark theme,
mobile-responsive with `Sheet` nav. Content rule: distill, link back to the
spec file, never duplicate protocol details.

## 15.7 Repo layout

```
web-guide/
  package.json            # React+Vite app (Node 20)
  vite.config.ts          # base: '/<repo>/' for project Pages
  components.json         # shadcn config (per skill)
  src/
    App.tsx               # HashRouter + layout shell
    content/*.mdx         # one file per route above
    components/           # shadcn-owned + custom (StepBlock, Terminal, …)
    lib/                  # search index, nav tree
  public/                 # screenshots, favicon
```

`web-guide/` is a docs app, not engine code: exempt from the Rust
`constants.rs`/`utils.rs` conventions in `AGENTS.md` §6.3; follows the
shadcn skill rules instead. Never commit `web-guide/dist/` or
`node_modules/` (extend `.gitignore` at scaffold time).

## 15.8 Deployment via GitHub Actions (DECIDED)

New workflow `.github/workflows/web-guide.yml` (alongside `ci.yml` /
`release.yml`; dual-OS matrix not needed — docs build on Linux only):

*   **Triggers:** push to `main` with paths `web-guide/**` + the workflow
    file itself; `workflow_dispatch` for manual rebuilds. PRs touching
    `web-guide/**` get a build-only check job (no deploy).
*   **Deploy job (`ubuntu-latest`):** `npm ci` → `npm run build` →
    `actions/upload-pages-artifact` (`web-guide/dist`) →
    `actions/deploy-pages`. Permissions `pages: write` + `id-token: write`,
    `environment: github-pages`, concurrency guard so runs don't overlap.
*   **Config:** `vite.config.ts` `base` set to the project-Pages subpath;
    `HashRouter` so deep links work without server rewrites.
*   **Versioning:** site versions with the repo (no separate release
    process); each deploy reflects green `main`.

## 15.9 Build order
1.  Scaffold Vite + TS + Tailwind + `components.json` (`shadcn init` flow
    per skill), `.gitignore` entries, smoke `npm run build`.
2.  Layout shell: sidebar, search stub, theme, hash routing.
3.  Custom MDX components: `StepBlock`, `Terminal`, `CodeTabs`, `Callout`
    (on shadcn `Alert`), `Mermaid`.
4.  Chapters in journey order (home → getting-started → … → faq),
    one route at a time, each verified with `npm run build`.
5.  Search index, polish pass (motion reveals, mobile nav), screenshots.
6.  `web-guide.yml` workflow + first Pages deploy; link from root `README.md`.

## 15.10 Theme tokens (DECIDED: terminal-green, dark + light)

*   **Dark (default):** near-black `--background`, phosphor terminal-green
    `--primary`, muted grays for `--muted`/`--sidebar-*`; custom pairs
    `--warning`/`--warning-foreground` (amber, Windows/ConPTY callouts) and
    `--info`/`--info-foreground` (DECIDED-note callouts).
*   **Light:** paper `--background`, deep-green `--primary` (same hue family,
    contrast-safe); custom pairs retuned for light.
*   **Mechanics:** class-based `.dark` toggle, persisted in `localStorage`,
    dark default. Scaffold via `npx shadcn@latest init --preset <dark-preset>`
    then override `--primary`/accent to terminal-green; register custom
    colors via `@theme inline` (Tailwind v4) or `tailwind.config.js` (v3).
*   **Type:** Inter (prose/UI) + JetBrains Mono (code/terminal), self-hosted
    via `@fontsource`; `--radius: 0.625rem`. Skill rules apply throughout:
    semantic utilities only, `flex` + `gap-*`, `cn()` for conditionals.

## 15.11 Homepage hero (animated)

Layout: two-column on desktop (copy left, terminal right), stacked on mobile;
subtle dot-grid background + green radial glow behind the terminal only.

*   **Copy block:** `Badge` eyebrow ("Black-box testing for terminal apps")
    → H1 "Playwright for Terminal Applications" (green gradient on
    "Terminal") → one-line subcopy (launch, drive, assert any TUI —
    no app changes) → CTA row: primary `Button` "Get started" (`/getting-started`)
    + outline `Button` "How it works" (`#how-it-works` anchor) with
    `data-icon` lucide arrows → stat strip (`9 protocol actions` ·
    `3 SDKs` · `Win + Linux CI`) as muted text with green numerals.
*   **`Terminal` window (the animation):** chrome bar (traffic dots +
    `tuilab — zsh` title), then a looping typed session driven by `motion`:
    `$ tuilab init` → scaffold lines → `$ tuilab run tests/e2e` →
    streaming `✓ smoke` / `✓ search-flow` / `2 passed, 0 failed` lines with
    staggered `whileInView`-style reveal + blinking block caret; green
    phosphor glow (`text-shadow`) in dark, solid deep-green in light.
    Loop with a pause on the result; restart on re-entry to viewport.
*   **Entrance choreography:** copy fades/slides in first (staggered children),
    terminal rises with a soft green shadow; all one-shot on load.
*   **Below the fold (`#how-it-works`):** three `Card`s (Write YAML → Run →
    Debug bundle) that fade-up on scroll, leading into the chapter grid.
*   **Accessibility/perf:** honor `prefers-reduced-motion` (render final
    terminal frame statically, skip typing); animation is CSS/`motion`
    transforms only, no layout thrash; terminal content is real text
    (selectable, screen-reader friendly).
