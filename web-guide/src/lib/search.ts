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
    text: 'YAML DSL steps press type wait_for_text resize assert_text assert_region snapshot wait_for_exit keys mouse click scroll ctrl alt fkeys timing key_delay input_delay tags skip focus budgets attachments',
  },
  {
    path: '/assertions-snapshots',
    title: 'Assertions & snapshots',
    text: 'assert taxonomy contains not_contains regex exact_text cursor exit_code crashed screen_changed role button textinput checkbox a11y accessibility tree snapshot golden styled cells sixel masking region deterministic retry timeout poll',
  },
  {
    path: '/recorder',
    title: 'Recorder',
    text: 'record capture replay smart waits codegen driven emit YAML target python js rust mouse click synthesis',
  },
  {
    path: '/recipes',
    title: 'Recipes',
    text: 'copy paste recipes examples search flow golden approval mask new file budgets startup step suite CI shard matrix artifacts upload parallel output-dir reports_dir isolation flaky retries history waterfall waterfall timing replay heisenbug role button assert_region cleanup wait_for_text a11y',
  },
  {
    path: '/reference',
    title: 'Architecture & reference',
    text: 'architecture protocol JSON-lines runtime PTY ConPTY configuration design source reports schema cross-platform packaging release layers',
  },
  {
    path: '/cli-reference',
    title: 'CLI reference',
    text: 'init run record report trace proto flags parallel tags shard retries resize-matrix step debug terminal tuilab.yaml exit codes CI junit html history flaky artifacts',
  },
  {
    path: '/mcp-agents',
    title: 'MCP & AI agents',
    text: 'tuilab-mcp tools tui_launch tui_press tui_type tui_screen tui_wait_for_text tui_assert tui_snapshot tui_resize tui_run_test tui_close allowlist cwd jail sensitive sessions audit',
  },
  {
    path: '/sdks',
    title: 'SDKs',
    text: 'Python JavaScript TypeScript Rust sidecar proto JSON-lines TuiTest Runner expect_text expectNotText press type snapshot resize close TUILAB_BIN binary resolution pytest node:test tokio',
  },
  {
    path: '/troubleshooting',
    title: 'Troubleshooting',
    text: 'install not found engine hang timeout flaky snapshot mask reports HTML junit trace replay failure bundle debug retries Windows ConPTY FORBIDDEN_COMMAND exit codes FAQ help',
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
  // The ESM bundle resolves non-enrich Document search to
  // [{ field, result: [ids] }] (the CJS bundle returns flat ids).
  // Handle both shapes.
  const raw = (await getIndex().search(q, { limit })) as unknown as Array<
    { result?: string[] } | string | string[]
  >
  const flat: string[] = raw.flatMap((group): string[] => {
    if (typeof group === 'string') return [group]
    if (Array.isArray(group)) return group
    return group.result ?? []
  })
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
