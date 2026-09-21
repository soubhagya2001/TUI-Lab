import type { ReactNode } from 'react'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { cn } from '@/lib/utils'

interface ExampleProps {
  title: string
  description?: string
  children: ReactNode
  className?: string
}

/**
 * Complete, runnable example (YAML suite, flow, config): visually distinct
 * from command snippets — green-tinted EXAMPLE card.
 */
export function Example({ title, description, children, className }: ExampleProps) {
  return (
    <Card className={cn('border-primary/30 bg-primary/[0.03]', className)}>
      <CardHeader className="space-y-1">
        <Badge className="w-fit">Example</Badge>
        <CardTitle className="text-lg">{title}</CardTitle>
        {description && <CardDescription className="text-base">{description}</CardDescription>}
      </CardHeader>
      <CardContent className="space-y-3">{children}</CardContent>
    </Card>
  )
}
