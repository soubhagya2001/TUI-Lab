import { MenuIcon, MoonIcon, SearchIcon, SquareTerminalIcon, SunIcon } from 'lucide-react'
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
      {next && (
        <Button asChild>
          <Link to={next.path}>{next.title} →</Link>
        </Button>
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
        <div className="mx-auto flex h-14 max-w-6xl items-center gap-3 px-4">
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
      <div className="mx-auto flex w-full max-w-6xl flex-1 gap-6 px-4">
        <aside className="hidden w-60 shrink-0 lg:block">
          <div className="sticky top-14 max-h-[calc(100svh-3.5rem)] overflow-y-auto py-4">
            <NavList />
          </div>
        </aside>
        <main className="min-w-0 flex-1 py-6">
          {current && pathname !== '/' && (
            <Breadcrumb className="mb-4">
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
          <Outlet />
          <Separator className="mt-12" />
          <PrevNext />
          {current && current.sources.length > 0 && (
            <p className="mt-6 text-xs text-muted-foreground">
              Distilled from: {current.sources.join(', ')} in the TUI Lab repo.
            </p>
          )}
        </main>
      </div>
      <footer className="border-t">
        <div className="mx-auto flex max-w-6xl flex-col gap-1 px-4 py-6 text-sm text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
          <span>TUI Lab developer guide — black-box testing for terminal apps.</span>
          <a
            className="text-primary underline underline-offset-4"
            href="https://github.com/soubhagya2001/TUI-Lab"
            target="_blank"
            rel="noreferrer"
          >
            GitHub repository
          </a>
        </div>
      </footer>
    </div>
  )
}
