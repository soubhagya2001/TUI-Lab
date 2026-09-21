import { MDXProvider } from '@mdx-js/react'
import { HashRouter, Route, Routes } from 'react-router-dom'
import { Layout } from '@/components/layout'
import { mdxComponents } from '@/components/mdx'
import { GUIDE_ROUTES } from '@/lib/nav'
import { ThemeProvider } from '@/lib/theme'
import { Home } from '@/pages/home'
import AssertionsSnapshots from '@/content/assertions-snapshots.mdx'
import CliReference from '@/content/cli-reference.mdx'
import GettingStarted from '@/content/getting-started.mdx'
import McpAgents from '@/content/mcp-agents.mdx'
import Recorder from '@/content/recorder.mdx'
import Sdks from '@/content/sdks.mdx'
import Troubleshooting from '@/content/troubleshooting.mdx'
import WritingTests from '@/content/writing-tests.mdx'

const CHAPTERS: Record<string, React.ComponentType> = {
  '/getting-started': GettingStarted,
  '/writing-tests': WritingTests,
  '/assertions-snapshots': AssertionsSnapshots,
  '/recorder': Recorder,
  '/cli-reference': CliReference,
  '/mcp-agents': McpAgents,
  '/sdks': Sdks,
  '/troubleshooting': Troubleshooting,
}

function Chapter({ path }: { path: string }) {
  const Component = CHAPTERS[path]
  if (!Component) return null
  return <Component />
}

export function App() {
  return (
    <ThemeProvider>
      <MDXProvider components={mdxComponents}>
        <HashRouter>
          <Routes>
            <Route element={<Layout />}>
              <Route index element={<Home />} />
              {GUIDE_ROUTES.filter((route) => route.path !== '/').map((route) => (
                <Route key={route.path} path={route.path} element={<Chapter path={route.path} />} />
              ))}
              <Route path="*" element={<Home />} />
            </Route>
          </Routes>
        </HashRouter>
      </MDXProvider>
    </ThemeProvider>
  )
}
