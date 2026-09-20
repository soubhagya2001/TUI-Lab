import { motion } from 'motion/react'
import { ArrowRightIcon, BookOpenIcon, FlaskConicalIcon, MousePointerClickIcon, ScrollTextIcon } from 'lucide-react'
import { Link } from 'react-router-dom'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Terminal, type TerminalLine } from '@/components/terminal'

const HERO_SCRIPT: TerminalLine[] = [
  { text: 'tuilab init', kind: 'command' },
  { text: 'Created tuilab.yaml + tests/smoke.yaml', kind: 'output' },
  { text: 'tuilab run tests/e2e', kind: 'command' },
  { text: '✓ smoke', kind: 'success' },
  { text: '✓ search-flow', kind: 'success' },
  { text: '2 passed, 0 failed', kind: 'output' },
]

const STATS = [
  { value: '9', label: 'protocol actions' },
  { value: '3', label: 'SDKs (Py / JS / Rust)' },
  { value: '2', label: 'OS in CI (Win + Linux)' },
]

const HOW_IT_WORKS = [
  {
    icon: ScrollTextIcon,
    title: 'Write YAML',
    text: 'Describe launches, keypresses, waits, and assertions in portable tui-lab/v1 suites. No app changes required.',
  },
  {
    icon: FlaskConicalIcon,
    title: 'Run',
    text: 'tuilab drives your app in a real PTY, syncing on wait_for_text — never sleep. Parallel fan-out included.',
  },
  {
    icon: MousePointerClickIcon,
    title: 'Debug the bundle',
    text: 'Every failure captures screens, history, and diffs. Re-render as JUnit or self-contained HTML.',
  },
]

export function Home() {
  return (
    <div className="flex flex-col gap-16">
      <section className="relative overflow-hidden rounded-2xl border">
        <div className="hero-grid absolute inset-0" aria-hidden="true" />
        <div className="relative grid items-center gap-8 p-6 sm:p-10 lg:grid-cols-2">
          <motion.div
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5 }}
            className="flex flex-col items-start gap-5"
          >
            <Badge variant="secondary">Black-box testing for terminal apps</Badge>
            <h1 className="text-4xl font-bold tracking-tight sm:text-5xl">
              Playwright for{' '}
              <span className="bg-gradient-to-r from-primary to-primary/60 bg-clip-text text-transparent">
                Terminal
              </span>{' '}
              Applications
            </h1>
            <p className="max-w-md text-lg text-muted-foreground">
              Launch, drive, and assert any TUI — Ratatui, Textual, Bubble Tea,
              plain scripts — from YAML, SDKs, or AI agents. No instrumentation needed.
            </p>
            <div className="flex flex-wrap gap-3">
              <Button asChild size="lg">
                <Link to="/getting-started">
                  Get started
                  <ArrowRightIcon data-icon="inline-end" />
                </Link>
              </Button>
              <Button asChild size="lg" variant="outline">
                <a href="#how-it-works">
                  <BookOpenIcon data-icon="inline-start" />
                  How it works
                </a>
              </Button>
            </div>
            <dl className="flex flex-wrap gap-x-8 gap-y-2">
              {STATS.map((stat) => (
                <div key={stat.label} className="flex items-baseline gap-2">
                  <dt className="sr-only">{stat.label}</dt>
                  <dd className="font-mono text-2xl font-bold text-primary">{stat.value}</dd>
                  <dd className="text-sm text-muted-foreground">{stat.label}</dd>
                </div>
              ))}
            </dl>
          </motion.div>
          <motion.div
            initial={{ opacity: 0, y: 24 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.55, delay: 0.15 }}
          >
            <Terminal title="tuilab — zsh" script={HERO_SCRIPT} />
          </motion.div>
        </div>
      </section>

      <section id="how-it-works" className="scroll-mt-20">
        <h2 className="mb-6 text-2xl font-bold tracking-tight">How it works</h2>
        <div className="grid gap-4 sm:grid-cols-3">
          {HOW_IT_WORKS.map((item, i) => (
            <motion.div
              key={item.title}
              initial={{ opacity: 0, y: 20 }}
              whileInView={{ opacity: 1, y: 0 }}
              viewport={{ once: true, margin: '-60px' }}
              transition={{ duration: 0.4, delay: i * 0.1 }}
            >
              <Card className="h-full">
                <CardHeader>
                  <item.icon className="mb-2 text-primary" data-icon="inline-start" />
                  <CardTitle>{item.title}</CardTitle>
                  <CardDescription className="text-base">{item.text}</CardDescription>
                </CardHeader>
              </Card>
            </motion.div>
          ))}
        </div>
      </section>
    </div>
  )
}
