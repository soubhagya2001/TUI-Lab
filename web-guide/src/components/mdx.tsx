import type { ComponentProps } from 'react'
import { CodeBlock } from '@/components/code-block'
import { cn } from '@/lib/utils'

function H1(props: ComponentProps<'h1'>) {
  return <h1 {...props} className="mb-4 text-3xl font-bold tracking-tight text-foreground sm:text-4xl" />
}

function H2(props: ComponentProps<'h2'>) {
  return (
    <h2
      {...props}
      className="mb-0 mt-4 scroll-mt-24 border-b pb-2 text-2xl font-semibold tracking-tight text-foreground"
    />
  )
}

function H3(props: ComponentProps<'h3'>) {
  return <h3 {...props} className="mb-0 mt-2 text-lg font-semibold text-foreground" />
}

function P(props: ComponentProps<'p'>) {
  return <p {...props} className="my-0 leading-7 text-foreground/90 [&_a]:text-primary [&_a]:underline" />
}

function A(props: ComponentProps<'a'>) {
  return <a {...props} className="text-primary underline underline-offset-4" />
}

function Ul(props: ComponentProps<'ul'>) {
  return <ul {...props} className="my-0 flex list-disc flex-col gap-1.5 pl-6 marker:text-primary" />
}

function Ol(props: ComponentProps<'ol'>) {
  return <ol {...props} className="my-0 flex list-decimal flex-col gap-1.5 pl-6 marker:text-primary" />
}

function Li(props: ComponentProps<'li'>) {
  return <li {...props} className="leading-7 text-foreground/90" />
}

function InlineCode(props: ComponentProps<'code'>) {
  return (
    <code
      {...props}
      className="rounded bg-muted px-1.5 py-0.5 font-mono text-[0.85em] text-foreground"
    />
  )
}

function Table(props: ComponentProps<'table'>) {
  return <table {...props} className="w-full text-left text-sm" />
}

function Th(props: ComponentProps<'th'>) {
  return <th {...props} className="border-b bg-muted/50 px-3 py-2 font-semibold" />
}

function Td(props: ComponentProps<'td'>) {
  return <td {...props} className="border-b px-3 py-2 align-top last:border-b-0" />
}

function Blockquote(props: ComponentProps<'blockquote'>) {
  return (
    <blockquote
      {...props}
      className="my-0 border-l-4 border-primary/60 bg-muted/40 py-1 pl-4 pr-2 italic text-foreground/90"
    />
  )
}

/** Element mapping for all MDX chapters. */
export const mdxComponents = {
  h1: H1,
  h2: H2,
  h3: H3,
  p: P,
  a: A,
  ul: Ul,
  ol: Ol,
  li: Li,
  pre: CodeBlock,
  code: InlineCode,
  table: Table,
  th: Th,
  td: Td,
  blockquote: Blockquote,
  wrapper: ({ children, ...props }: ComponentProps<'div'>) => (
    <div {...props} className={cn('flex flex-col gap-6')}>
      {children}
    </div>
  ),
}
