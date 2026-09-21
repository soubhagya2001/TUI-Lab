import { MailIcon, MenuIcon, MoonIcon, SearchIcon, SquareTerminalIcon, SunIcon } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { Link, NavLink, Outlet, useLocation } from 'react-router-dom'
import { Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbPage, BreadcrumbSeparator } from '@/components/ui/breadcrumb'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Separator } from '@/components/ui/separator'
import { Sheet, SheetContent, SheetTitle, SheetTrigger } from '@/components/ui/sheet'
import { GUIDE_ROUTES } from '@/lib/nav'
import { searchGuide } from '@/lib/search'
import { useTheme } from '@/lib/theme'
import { cn } from '@/lib/utils'

function NavList({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <nav aria-label="Guide chapters" className="flex flex-col gap-1 p-3">
      {GUIDE_ROUTES.map((route) => (
        <NavLink
          key={route.path}
          to={route.path}
          end={route.path === '/'}
          onClick={onNavigate}
          className={({ isActive }) =>
            cn(
              'rounded-lg px-3 py-2 text-sm transition-colors hover:bg-accent hover:text-accent-foreground',
              isActive
                ? 'bg-accent font-medium text-accent-foreground'
                : 'text-muted-foreground',
            )
          }
        >
          {route.title}
        </NavLink>
      ))}
    </nav>
  )
}

import type { SearchEntry } from '@/lib/search'

function SearchBox() {
  const [query, setQuery] = useState('')
  const [open, setOpen] = useState(false)
  const [results, setResults] = useState<SearchEntry[]>([])
  const boxRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    let cancelled = false
    if (query.trim().length < 2) {
      setResults([])
      return
    }
    const timer = setTimeout(() => {
      void searchGuide(query).then((hits) => {
        if (!cancelled) setResults(hits)
      })
    }, 150)
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [query])

  useEffect(() => {
    const onClick = (event: MouseEvent) => {
      if (boxRef.current && !boxRef.current.contains(event.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', onClick)
    return () => document.removeEventListener('mousedown', onClick)
  }, [])

  return (
    <div ref={boxRef} className="relative w-full max-w-xs">
      <SearchIcon className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
      <Input
        value={query}
        onChange={(event) => {
          setQuery(event.target.value)
          setOpen(true)
        }}
        onFocus={() => setOpen(true)}
        placeholder="Search the guide…"
        aria-label="Search the guide"
        className="pl-9"
      />
      {open && results.length > 0 && (
        <div className="absolute top-full z-50 mt-1 w-full overflow-hidden rounded-lg border bg-popover shadow-lg">
          {results.map((result) => (
            <Link
              key={result.path}
              to={result.path}
              onClick={() => {
                setOpen(false)
                setQuery('')
              }}
              className="block px-3 py-2 text-sm hover:bg-accent hover:text-accent-foreground"
            >
              <span className="font-medium">{result.title}</span>
              <span className="block truncate text-xs text-muted-foreground">{result.text}</span>
            </Link>
          ))}
        </div>
      )}
    </div>
  )
}

function PrevNext() {
  const { pathname } = useLocation()
  const index = GUIDE_ROUTES.findIndex((route) => route.path === pathname)
  const prev = index > 0 ? GUIDE_ROUTES[index - 1] : null
  const next = index >= 0 && index < GUIDE_ROUTES.length - 1 ? GUIDE_ROUTES[index + 1] : null
  if (!prev && !next) return null
  return (
    <div className="mt-12 flex items-center justify-between gap-4">
      {prev ? (
        <Button variant="outline" asChild>
          <Link to={prev.path}>← {prev.title}</Link>
        </Button>
      ) : (
        <span />
      )}
      {next ? (
        <span className="hidden text-center text-sm text-muted-foreground sm:block">
          Up next:{' '}
          <Link to={next.path} className="font-medium text-primary underline underline-offset-4">
            {next.title}
          </Link>
        </span>
      ) : (
        <span />
      )}
      {next ? (
        <Button asChild>
          <Link to={next.path}>{next.title} →</Link>
        </Button>
      ) : (
        <span />
      )}
    </div>
  )
}

export function Layout() {
  const { theme, toggle } = useTheme()
  const { pathname } = useLocation()
  const [sheetOpen, setSheetOpen] = useState(false)
  const current = GUIDE_ROUTES.find((route) => route.path === pathname)

  return (
    <div className="flex min-h-svh flex-col">
      <header className="sticky top-0 z-40 border-b bg-background/95 backdrop-blur">
        <div className="mx-auto flex h-14 max-w-[1440px] items-center gap-3 px-6">
          <Sheet open={sheetOpen} onOpenChange={setSheetOpen}>
            <SheetTrigger asChild className="lg:hidden">
              <Button variant="ghost" size="icon" aria-label="Open navigation">
                <MenuIcon />
              </Button>
            </SheetTrigger>
            <SheetContent side="left" className="w-72 p-0">
              <SheetTitle className="sr-only">Guide navigation</SheetTitle>
              <ScrollArea className="h-full">
                <NavList onNavigate={() => setSheetOpen(false)} />
              </ScrollArea>
            </SheetContent>
          </Sheet>
          <Link to="/" className="flex shrink-0 items-center gap-2 font-mono font-bold">
            <SquareTerminalIcon className="text-primary" data-icon="inline-start" />
            <span>
              TUI<span className="text-primary">Lab</span>
              <span className="ml-2 hidden text-xs font-normal text-muted-foreground sm:inline">
                Guide
              </span>
            </span>
          </Link>
          <div className="flex flex-1 justify-center">
            <SearchBox />
          </div>
          <Button variant="ghost" size="icon" onClick={toggle} aria-label={theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme'}>
            {theme === 'dark' ? <SunIcon /> : <MoonIcon />}
          </Button>
        </div>
      </header>
      <div className="mx-auto flex w-full max-w-[1440px] flex-1 gap-6 px-6">
        <aside className="hidden w-64 shrink-0 lg:block">
          <div className="sticky top-14 max-h-[calc(100svh-3.5rem)] overflow-y-auto py-4">
            <NavList />
          </div>
        </aside>
        <main className="min-w-0 flex-1 py-6">
          {current && pathname !== '/' && (
            <Breadcrumb className="mb-6">
              <BreadcrumbList>
                <BreadcrumbItem>
                  <BreadcrumbLink asChild>
                    <Link to="/">Home</Link>
                  </BreadcrumbLink>
                </BreadcrumbItem>
                <BreadcrumbSeparator />
                <BreadcrumbItem>
                  <BreadcrumbPage>{current.title}</BreadcrumbPage>
                </BreadcrumbItem>
              </BreadcrumbList>
            </Breadcrumb>
          )}
          {/* Chapter body: MDX top-level nodes become direct DOM children
              (fragments render no wrapper), so the flex gap here guarantees
              rhythm even if the MDX `wrapper` mapping is bypassed. */}
          <div className="chapter-body flex flex-col gap-6">
            <Outlet />
          </div>
          <Separator className="mt-12" />
          <PrevNext />
        </main>
      </div>
      <footer className="border-t">
        <div className="mx-auto flex max-w-[1440px] flex-col gap-4 px-6 py-8 sm:flex-row sm:items-start sm:justify-between">
          <div className="flex flex-col gap-1 text-sm text-muted-foreground">
            <span className="font-semibold text-foreground">TUI Lab developer guide</span>
            <span>Black-box testing for terminal apps.</span>
          </div>
          <div className="flex flex-col gap-2">
            <span className="text-sm font-semibold">Contact us</span>
            <div className="flex flex-wrap gap-2">
              <Button variant="outline" size="sm" asChild>
                <a href="mailto:soubhagyaprusty36@gmail.com">
                  <MailIcon data-icon="inline-start" />
                  Email
                </a>
              </Button>
              <Button variant="outline" size="sm" asChild>
                <a
                  href="https://linkedin.com/in/soubhagya-prusty-5424811b6"
                  target="_blank"
                  rel="noreferrer"
                >
                  LinkedIn
                </a>
              </Button>
              <Button variant="outline" size="sm" asChild>
                <a href="https://github.com/soubhagya2001" target="_blank" rel="noreferrer">
                  GitHub
                </a>
              </Button>
            </div>
          </div>
        </div>
      </footer>
    </div>
  )
}
