import { useState, type ReactNode } from 'react'
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { cn } from '@/lib/utils'

interface CalloutProps {
  title: string
  children: ReactNode
  /** Maps to semantic token pairs: info (DECIDED notes), warning (Windows/ConPTY gotchas). */
  tone?: 'info' | 'warning'
  className?: string
}

/** Colored callout built on shadcn Alert — never a custom styled div. */
export function Callout({ title, children, tone = 'info', className }: CalloutProps) {
  return (
    <Alert
      className={cn(
        tone === 'warning'
          ? 'border-warning/50 bg-warning text-warning-foreground'
          : 'border-info/50 bg-info text-info-foreground',
        className,
      )}
    >
      <AlertTitle>{title}</AlertTitle>
      <AlertDescription className="[&_a]:underline">{children}</AlertDescription>
    </Alert>
  )
}

export interface CodeTab {
  label: string
  value: string
  content: ReactNode
}

interface CodeTabsProps {
  tabs: CodeTab[]
  defaultValue?: string
  className?: string
}

/** Tabbed code samples (e.g. Python / JavaScript / Rust SDK sketches). */
export function CodeTabs({ tabs, defaultValue, className }: CodeTabsProps) {
  const [value, setValue] = useState(defaultValue ?? tabs[0]?.value)
  return (
    <Tabs value={value} onValueChange={setValue} className={className}>
      <TabsList>
        {tabs.map((tab) => (
          <TabsTrigger key={tab.value} value={tab.value}>
            {tab.label}
          </TabsTrigger>
        ))}
      </TabsList>
      {tabs.map((tab) => (
        <TabsContent key={tab.value} value={tab.value}>
          {tab.content}
        </TabsContent>
      ))}
    </Tabs>
  )
}
