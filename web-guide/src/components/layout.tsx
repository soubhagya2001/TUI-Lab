import { MailIcon, MenuIcon, MoonIcon, SearchIcon, SquareTerminalIcon, SunIcon } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { Link, NavLink, Outlet, useLocation, useNavigate } from 'react-router-dom'
import { Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbPage, BreadcrumbSeparator } from '@/components/ui/breadcrumb'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Separator } from '@/components/ui/separator'
import { Sheet, SheetContent, SheetTitle, SheetTrigger } from '@/components/ui/sheet'
import { GUIDE_ROUTES, NAV_SECTIONS } from '@/lib/nav'
import { searchGuide } from '@/lib/search'
import { useTheme } from '@/lib/theme'
import { cn } from '@/lib/utils'

function NavList({ onNavigate }: { onNavigate?: () => void }) {
  return (
    <nav aria-label="Guide chapters" className="flex flex-col gap-4 p-3">
      {NAV_SECTIONS.map((section) => (
        <div key={section.label} className="flex flex-col gap-1">
          <span className="px-3 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
            {section.label}
          </span>
          {section.routes.map((route) => (
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
        </div>
      ))}
    </nav>
  )
}

import type { SearchEntry } from '@/lib/search'

function highlightMatch(title: string, query: string) {
  const text = query.trim()
  if (text.length < 2) return title
  const index = title.toLowerCase().indexOf(text.toLowerCase())
  if (index < 0) return title
  return (
    <>
      {title.slice(0, index)}
      <mark className="rounded-sm bg-primary/25 text-inherit">
        {title.slice(index, index + text.length)}
      </mark>
      {title.slice(index + text.length)}
    </>
  )
}

function SearchBox() {
  const [query, setQuery] = useState('')
  const [open, setOpen] = useState(false)
  const [results, setResults] = useState<SearchEntry[]>([])
  const [active, setActive] = useState(0)
  const [searched, setSearched] = useState(false)
  const boxRef = useRef<HTMLDivElement>(null)
  const navigate = useNavigate()

  useEffect(() => {
    let cancelled = false
    setActive(0)
    if (query.trim().length < 2) {
      setResults([])
      setSearched(false)
      return
    }
    const timer = setTimeout(() => {
      void searchGuide(query).then((hits) => {
        if (cancelled) return
        setResults(hits)
        setSearched(true)
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

  const go = (path: string) => {
    setOpen(false)
    setQuery('')
    navigate(path)
  }

  // The panel mounts only once there is something to show — results or
  // the searched empty state — so it never flashes as an empty box while
  // the debounced search is still running.
  const showDropdown = open && query.trim().length >= 2 && (results.length > 0 || searched)

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
        onKeyDown={(event) => {
          if (event.key === 'ArrowDown' && results.length > 0) {
            event.preventDefault()
            setOpen(true)
            setActive((index) => (index + 1) % results.length)
          } else if (event.key === 'ArrowUp' && results.length > 0) {
            event.preventDefault()
            setActive((index) => (index - 1 + results.length) % results.length)
          } else if (event.key === 'Enter' && open && results[active]) {
            go(results[active].path)
          } else if (event.key === 'Escape') {
            setOpen(false)
          }
        }}
        placeholder="Search the guide…"
        aria-label="Search the guide"
        role="combobox"
        aria-expanded={showDropdown}
        aria-controls="guide-search-results"
        aria-activedescendant={results[active] ? `guide-search-${results[active].path}` : undefined}
        className="pl-9"
      />
      {showDropdown && (
        <div
          id="guide-search-results"
          data-testid="search-results"
          role="listbox"
          aria-label="Search suggestions"
          className="absolute top-full z-50 mt-1 w-full overflow-hidden rounded-lg border bg-popover shadow-lg"
        >
          {results.map((result, index) => (
            <Link
              key={result.path}
              id={`guide-search-${result.path}`}
              role="option"
              aria-selected={index === active}
              to={result.path}
              onMouseEnter={() => setActive(index)}
              onClick={() => go(result.path)}
              className={index === active ? 'block bg-accent px-3 py-2 text-sm text-accent-foreground' : 'block px-3 py-2 text-sm hover:bg-accent hover:text-accent-foreground'}
            >
              <span className="font-medium">{highlightMatch(result.title, query)}</span>
              <span className="block truncate text-xs text-muted-foreground">{result.text}</span>
            </Link>
          ))}
          {searched && results.length === 0 && (
            <p className="px-3 py-2 text-sm text-muted-foreground">
              No matches for “{query.trim()}”.
            </p>
          )}
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
