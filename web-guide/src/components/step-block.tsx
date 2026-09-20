import type { ReactNode } from 'react'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { cn } from '@/lib/utils'

interface StepBlockProps {
  step: number
  title: string
  children: ReactNode
  tag?: string
  className?: string
}

/** Numbered guide step: explanation + commands + expected output. */
export function StepBlock({ step, title, children, tag, className }: StepBlockProps) {
  return (
    <Card className={cn('overflow-hidden', className)}>
      <CardHeader className="flex flex-row items-center gap-3 space-y-0">
        <span className="flex size-8 shrink-0 items-center justify-center rounded-full bg-primary font-mono text-sm font-bold text-primary-foreground">
          {step}
        </span>
        <div className="flex flex-1 flex-wrap items-center gap-2">
          <CardTitle className="text-lg">{title}</CardTitle>
          {tag && <Badge variant="secondary">{tag}</Badge>}
        </div>
      </CardHeader>
      <CardContent className="space-y-3">{children}</CardContent>
    </Card>
  )
}

export function StepDescription({ children }: { children: ReactNode }) {
  return <CardDescription className="text-base">{children}</CardDescription>
}
