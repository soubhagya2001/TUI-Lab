export interface GuideRoute {
  path: string
  title: string
  description: string
  sources: string[]
}

export const GUIDE_ROUTES: GuideRoute[] = [
  {
    path: '/',
    title: 'Home',
    description: 'What TUI Lab is and the 60-second quickstart.',
    sources: ['docs/01-vision-scope.md', 'docs/14-implementation-roadmap.md'],
  },
  {
    path: '/getting-started',
    title: 'Getting started',
    description: 'Install, tuilab init, first green run.',
    sources: ['docs/07-cli-reference.md', 'docs/14-implementation-roadmap.md'],
  },
  {
    path: '/writing-tests',
    title: 'Writing tests',
    description: 'YAML DSL step reference with examples.',
    sources: ['docs/05-test-definition-dsl.md'],
  },
  {
    path: '/assertions-snapshots',
    title: 'Assertions & snapshots',
    description: 'Assertion taxonomy, masking, retry semantics.',
    sources: ['docs/06-assertion-snapshot-engine.md'],
  },
  {
    path: '/recorder',
    title: 'Recorder',
    description: 'Record workflows with smart waits.',
    sources: ['docs/10-recorder-and-ai.md'],
  },
  {
    path: '/cli-reference',
    title: 'CLI reference',
    description: 'Commands, flags, tuilab.yaml, exit codes.',
    sources: ['docs/07-cli-reference.md'],
  },
  {
    path: '/mcp-agents',
    title: 'MCP & AI agents',
    description: 'Nine MCP tools, Modes A/B, security.',
    sources: ['docs/08-mcp-server-spec.md'],
  },
  {
    path: '/sdks',
    title: 'SDKs',
    description: 'Python, JavaScript, and Rust setup + sketches.',
    sources: ['docs/09-sdk-integration-guide.md'],
  },
  {
    path: '/ci-troubleshooting',
    title: 'CI & troubleshooting',
    description: 'CI example, reports, failure bundle, FAQ.',
    sources: ['docs/11-ci-reporting-debugging.md', 'docs/12-cross-platform-strategy.md'],
  },
]
