import { CheckIcon, CopyIcon } from 'lucide-react'
import { useCallback, useState, type ReactNode } from 'react'
import { Badge } from '@/components/ui/badge'
import { cn } from '@/lib/utils'

/** Shared copy-to-clipboard state. */
export function useCopy() {
  const [copied, setCopied] = useState(false)
  const copyText = useCallback(async (text: string) => {
    if (!text) return
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 1600)
    } catch {
      // Clipboard unavailable (e.g. insecure context); no-op.
    }
  }, [])
  return { copied, copyText }
}

/** Small copy button with Copied feedback. Text is read at click time. */
export function CopyButton({ getText, className }: { getText: () => string; className?: string }) {
  const { copied, copyText } = useCopy()
  return (
    <button
      type="button"
      onClick={() => void copyText(getText())}
      aria-label={copied ? 'Copied' : 'Copy code'}
      className={cn(
        'flex items-center gap-1 rounded-md px-1.5 py-1 text-xs text-muted-foreground transition-colors hover:text-foreground',
        className,
      )}
    >
      {copied ? <CheckIcon data-icon="inline-start" /> : <CopyIcon data-icon="inline-start" />}
      {copied ? 'Copied' : 'Copy'}
    </button>
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

/** Copyable code snippet for tabbed content (install commands, configs). */
export function Snippet({ label, code, className }: { label: string; code: string; className?: string }) {
  return (
    <div className={cn('overflow-hidden rounded-xl border bg-card', className)}>
      <div className="flex items-center justify-between border-b bg-muted/40 px-3 py-1.5">
        <Badge variant="outline" className="font-mono text-[11px] uppercase tracking-wide">
          {label}
        </Badge>
        <CopyButton getText={() => code} />
      </div>
      <pre className="overflow-x-auto p-4 font-mono text-sm leading-relaxed">
        <code>{code}</code>
      </pre>
    </div>
  )
}
