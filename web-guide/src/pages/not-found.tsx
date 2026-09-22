import { Link } from 'react-router-dom'
import { Button } from '@/components/ui/button'
import { GUIDE_ROUTES } from '@/lib/nav'

/** Unknown hash route: search-friendly 404 inside the guide layout. */
export function NotFound() {
  return (
    <div className="flex flex-col items-start gap-4 py-8">
      <p className="font-mono text-5xl font-bold text-primary">404</p>
      <h1 className="text-2xl font-bold tracking-tight">That page isn’t in this guide</h1>
      <p className="max-w-md text-muted-foreground">
        Use the search box above, or jump straight into one of these chapters:
      </p>
      <div className="flex flex-wrap gap-2">
        {GUIDE_ROUTES.filter((route) => route.path !== '/')
          .slice(0, 4)
          .map((route) => (
            <Button key={route.path} variant="outline" size="sm" asChild>
              <Link to={route.path}>{route.title}</Link>
            </Button>
          ))}
      </div>
      <Button asChild>
        <Link to="/">Back to Home</Link>
      </Button>
    </div>
  )
}
