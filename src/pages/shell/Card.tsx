import type { ReactNode } from 'react'
import { cn } from '@/lib/utils'

/** Compose / Settings 外层卡片。 */
export function Card({ children }: { children: ReactNode }) {
  return (
    <div
      className={cn(
        'min-w-[540px] rounded-lg bg-card text-card-foreground ring-1 ring-foreground/10',
        'shadow-[0_1px_2px_rgb(0_0_0/0.16)] dark:shadow-[0_1px_2px_rgb(0_0_0/0.55)]',
      )}
    >
      {children}
    </div>
  )
}
