export interface GuideRoute {
  path: string
  title: string
  description: string
}

export interface NavSection {
  label: string
  routes: GuideRoute[];
}

export const GUIDE_ROUTES: GuideRoute[] = [
  {
    path: '/',
    title: 'Home',
    description: 'What TUI Lab is and the 60-second quickstart.',
  },
  {
    path: '/getting-started',
    title: 'Getting started',
    description: 'Install, tuilab init, first green run.',
  },
  {
    path: '/writing-tests',
    title: 'Writing tests',
    description: 'YAML DSL step reference with examples.',
  },
  {
    path: '/assertions-snapshots',
    title: 'Assertions & snapshots',
    description: 'Assertion taxonomy, masking, retry semantics.',
  },
  {
    path: '/recorder',
    title: 'Recorder',
    description: 'Record workflows with smart waits.',
  },
  {
    path: '/recipes',
    title: 'Recipes',
    description: 'Copy-paste patterns: search flows, goldens, budgets, CI shards.',
  },
  {
    path: '/cli-reference',
    title: 'CLI reference',
    description: 'Commands, flags, tuilab.yaml, exit codes.',
  },
  {
    path: '/mcp-agents',
    title: 'MCP & AI agents',
    description: 'Ten MCP tools, Modes A/B, security, audit log.',
  },
  {
    path: '/sdks',
    title: 'SDKs',
    description: 'Python, JavaScript, and Rust setup + sketches.',
  },
  {
    path: '/troubleshooting',
    title: 'Troubleshooting',
    description: 'Install problems, hangs, flakes, reports, FAQ.',
  },
]

function section(label: string, paths: string[]): NavSection {
  return {
    label,
    routes: GUIDE_ROUTES.filter((route) => paths.includes(route.path)),
  }
}

export const NAV_SECTIONS: NavSection[] = [
  section('Start', ['/', '/getting-started']),
  section('Write', ['/writing-tests', '/assertions-snapshots', '/recorder', '/recipes']),
  section('Reference', ['/cli-reference', '/mcp-agents', '/sdks']),
  section('Help', ['/troubleshooting']),
]
