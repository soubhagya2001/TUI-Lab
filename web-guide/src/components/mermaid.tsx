import { useEffect, useRef } from 'react'
import mermaid from 'mermaid'

let initialized = false

function ensureInitialized() {
  if (initialized) return
  mermaid.initialize({ startOnLoad: false, theme: 'base' })
  initialized = true
}

interface MermaidProps {
  chart: string
  title?: string
}

/** Renders a Mermaid diagram (architecture from docs/02). */
export function Mermaid({ chart, title }: MermaidProps) {
  const ref = useRef<HTMLDivElement>(null)

  useEffect(() => {
    ensureInitialized()
    const node = ref.current
    if (!node) return
    let cancelled = false
    mermaid
      .render(`mermaid-${Math.random().toString(36).slice(2)}`, chart)
      .then(({ svg }) => {
        if (!cancelled) node.innerHTML = svg
      })
      .catch(() => {
        if (!cancelled) node.innerHTML = '<p>Diagram failed to render.</p>'
      })
    return () => {
      cancelled = true
    }
  }, [chart])

  return (
    <figure className="overflow-x-auto rounded-xl border bg-card p-4">
      {title && <figcaption className="mb-2 text-sm text-muted-foreground">{title}</figcaption>}
      <div ref={ref} aria-label={title ?? 'Mermaid diagram'} />
    </figure>
  )
}
