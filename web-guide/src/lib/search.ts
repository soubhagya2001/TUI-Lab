import { Document } from 'flexsearch'

export interface SearchEntry {
  path: string
  title: string
  text: string
}

interface StoredEntry extends SearchEntry {
  [key: string]: string
}

const CORPUS: StoredEntry[] = [
  {
    path: '/',
    title: 'Home',
    text: 'TUI Lab black-box testing terminal applications Playwright quickstart install run',
  },
  {
    path: '/getting-started',
    title: 'Getting started',
    text: 'install cargo build tuilab init scaffold smoke run first test yaml config',
  },
  {
    path: '/writing-tests',
    title: 'Writing tests',
    text: 'YAML DSL steps launch press type wait_for_text screen assert snapshot resize close session',
  },
  {
    path: '/assertions-snapshots',
    title: 'Assertions & snapshots',
    text: 'assert taxonomy contains matches snapshot golden masking deterministic retry timeout poll',
  },
  {
    path: '/recorder',
    title: 'Recorder',
    text: 'record capture replay smart waits codegen driven emit YAML',
  },
  {
    path: '/cli-reference',
    title: 'CLI reference',
    text: 'init run record report debug step proto flags parallel tuilab.yaml exit codes CI',
  },
  {
    path: '/mcp-agents',
    title: 'MCP & AI agents',
    text: 'tuilab-mcp tools tui_launch tui_press tui_type tui_wait tui_screen tui_assert tui_snapshot tui_resize tui_close allowlist cwd jail sensitive sessions',
  },
  {
    path: '/sdks',
    title: 'SDKs',
    text: 'Python JavaScript TypeScript Rust sidecar proto JSON-lines pytest node:test tokio',
  },
  {
    path: '/ci-troubleshooting',
    title: 'CI & troubleshooting',
    text: 'GitHub Actions JUnit HTML failure bundle debug ConPTY Windows Linux resize mouse timing flaky',
  },
]

let index: Document<StoredEntry, true> | null = null

function getIndex() {
  if (!index) {
    index = new Document<StoredEntry, true>({
      document: { id: 'path', index: ['title', 'text'], store: true },
    })
    for (const entry of CORPUS) index.add(entry)
  }
  return index
}

export async function searchGuide(query: string, limit = 7): Promise<SearchEntry[]> {
  const q = query.trim()
  if (q.length < 2) return []
  const raw = (await getIndex().search(q, { limit })) as unknown as (string | string[])[]
  const flat: string[] = raw.flatMap((id) => (Array.isArray(id) ? id : [id]))
  const seen = new Set<string>()
  const out: SearchEntry[] = []
  for (const id of flat) {
    if (seen.has(id)) continue
    seen.add(id)
    const entry = CORPUS.find((item) => item.path === id)
    if (entry) out.push(entry)
  }
  return out
}
