import { CheckIcon, CopyIcon } from 'lucide-react'
import { useRef, useState, type ComponentProps } from 'react'
import { cn } from '@/lib/utils'

/** Code block with copy button; used for MDX `pre`. */
export function CodeBlock({ children, className, ...props }: ComponentProps<'pre'>) {
  const [copied, setCopied] = useState(false)
  const preRef = useRef<HTMLPreElement>(null)

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
    <div className="group relative overflow-hidden rounded-xl border bg-card">
      <button
        type="button"
        onClick={copy}
        aria-label={copied ? 'Copied' : 'Copy code'}
        className="absolute right-2 top-2 rounded-md border bg-background p-1.5 text-muted-foreground opacity-0 transition-opacity hover:text-foreground focus:opacity-100 group-hover:opacity-100"
      >
        {copied ? <CheckIcon data-icon="inline-start" /> : <CopyIcon data-icon="inline-start" />}
      </button>
      <pre ref={preRef} {...props} className={cn('[&_code]:bg-transparent [&_code]:p-0', className)}>
        {children}
      </pre>
    </div>
  )
}
