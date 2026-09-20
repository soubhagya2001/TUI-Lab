import { useEffect, useState } from 'react'
import { cn } from '@/lib/utils'

export interface TerminalLine {
  text: string
  kind: 'command' | 'output' | 'success'
}

interface TerminalProps {
  title?: string
  script: TerminalLine[]
  /** ms per character while typing commands */
  charDelay?: number
  /** pause after the full script before looping */
  endPause?: number
  loop?: boolean
  className?: string
}

const OUTPUT_DELAY = 320

function useReducedMotion() {
  const [reduced, setReduced] = useState(false)
  useEffect(() => {
    const query = window.matchMedia('(prefers-reduced-motion: reduce)')
    setReduced(query.matches)
    const onChange = (event: MediaQueryListEvent) => setReduced(event.matches)
    query.addEventListener('change', onChange)
    return () => query.removeEventListener('change', onChange)
  }, [])
  return reduced
}

/**
 * Animated terminal: types `$` commands, streams output lines, loops.
 * Renders the final frame statically when reduced motion is preferred.
 */
export function Terminal({
  title = 'tuilab — zsh',
  script,
  charDelay = 45,
  endPause = 2600,
  loop = true,
  className,
}: TerminalProps) {
  const reducedMotion = useReducedMotion()
  const [visibleCount, setVisibleCount] = useState(reducedMotion ? script.length : 0)
  const [typedChars, setTypedChars] = useState(0)

  useEffect(() => {
    if (reducedMotion) {
      setVisibleCount(script.length)
      return
    }
    let cancelled = false
    let timer: ReturnType<typeof setTimeout>

    const run = async () => {
      setVisibleCount(0)
      setTypedChars(0)
      for (let i = 0; i < script.length; i++) {
        if (cancelled) return
        const line = script[i]
        if (line.kind === 'command') {
          for (let c = 1; c <= line.text.length; c++) {
            if (cancelled) return
            setTypedChars(c)
            await new Promise((resolve) => {
              timer = setTimeout(resolve, charDelay)
            })
          }
          setVisibleCount(i + 1)
          setTypedChars(0)
        } else {
          await new Promise((resolve) => {
            timer = setTimeout(resolve, OUTPUT_DELAY)
          })
          if (cancelled) return
          setVisibleCount(i + 1)
        }
      }
      if (loop && !cancelled) {
        await new Promise((resolve) => {
          timer = setTimeout(resolve, endPause)
        })
        if (!cancelled) void run()
      }
    }

    void run()
    return () => {
      cancelled = true
      clearTimeout(timer)
    }
  }, [script, charDelay, endPause, loop, reducedMotion])

  return (
    <div
      role="img"
      aria-label="Terminal session showing tuilab commands and results"
      className={cn(
        'overflow-hidden rounded-xl border bg-card text-left shadow-2xl shadow-primary/10',
        className,
      )}
    >
      <div className="flex items-center gap-2 border-b px-4 py-2.5">
        <span className="size-3 rounded-full bg-destructive/70" />
        <span className="size-3 rounded-full bg-warning/70" />
        <span className="size-3 rounded-full bg-primary/70" />
        <span className="ml-2 truncate font-mono text-xs text-muted-foreground">{title}</span>
      </div>
      <div className="min-h-56 space-y-1.5 p-4 font-mono text-sm leading-relaxed">
        {script.slice(0, visibleCount).map((line, i) => (
          <p
            key={i}
            className={cn(
              'whitespace-pre-wrap break-words',
              line.kind === 'command' && 'text-foreground',
              line.kind === 'output' && 'text-muted-foreground',
              line.kind === 'success' && 'text-primary terminal-glow',
            )}
          >
            {line.kind === 'command' ? `$ ${line.text}` : line.text}
          </p>
        ))}
        {!reducedMotion && visibleCount < script.length && script[visibleCount]?.kind === 'command' && (
          <p className="whitespace-pre-wrap text-foreground">
            {`$ ${script[visibleCount].text.slice(0, typedChars)}`}
            <span className="caret-blink text-primary">▊</span>
          </p>
        )}
        {visibleCount >= script.length && (
          <p className="text-foreground">
            $ <span className="caret-blink text-primary">▊</span>
          </p>
        )}
      </div>
    </div>
  )
}
