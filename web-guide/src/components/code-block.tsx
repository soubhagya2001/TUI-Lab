import { CheckIcon, CopyIcon } from 'lucide-react'
import { Children, isValidElement, useRef, useState, type ComponentProps, type ReactNode } from 'react'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

function languageOf(children: ReactNode): string | null {
  const child = Children.only(children)
  if (isValidElement<{ className?: string }>(child)) {
    const match = /language-([\w-]+)/.exec(child.props.className ?? '')
    if (match) return match[1].replace('text', 'output')
  }
  return null
}

/** Labeled code snippet with copy button; used for MDX `pre`. */
export function CodeBlock({ children, className, ...props }: ComponentProps<'pre'>) {
  const [copied, setCopied] = useState(false)
  const preRef = useRef<HTMLPreElement>(null)
  let language: string | null = null
  try {
    language = languageOf(children)
  } catch {
    language = null
  }

  const copy = async () => {
    const text = preRef.current?.textContent ?? ''
    if (!text) return
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 1600)
    } catch {
      // Clipboard unavailable (e.g. insecure context); no-op.
    }
  }

  return (
    <div className="overflow-hidden rounded-xl border bg-card">
      <div className="flex items-center justify-between border-b bg-muted/40 px-3 py-1.5">
        <Badge variant="outline" className="font-mono text-[11px] uppercase tracking-wide">
          {language ?? 'code'}
        </Badge>
        <button
          type="button"
          onClick={copy}
          aria-label={copied ? 'Copied' : 'Copy code'}
          className="flex items-center gap-1 rounded-md px-1.5 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground"
        >
          {copied ? <CheckIcon data-icon="inline-start" /> : <CopyIcon data-icon="inline-start" />}
          {copied ? 'Copied' : 'Copy'}
        </button>
      </div>
      <pre ref={preRef} {...props} className={cn('[&_code]:bg-transparent [&_code]:p-0', className)}>
        {children}
      </pre>
    </div>
  )
}

/** Static terminal output: dark, mono, clearly not a command to run. */
export function OutputBlock({ children, title = 'output', className }: { children: ReactNode; title?: string; className?: string }) {
  return (
    <div className={cn('overflow-hidden rounded-xl border border-primary/25 bg-zinc-950 dark:bg-black', className)}>
      <div className="border-b border-white/10 px-3 py-1.5">
        <Badge variant="outline" className="border-primary/40 font-mono text-[11px] uppercase tracking-wide text-primary">
          {title}
        </Badge>
      </div>
      <div className="whitespace-pre-wrap break-words p-4 font-mono text-sm leading-relaxed text-zinc-100">
        {children}
      </div>
    </div>
  )
}
