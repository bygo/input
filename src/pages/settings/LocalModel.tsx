import { listen } from '@tauri-apps/api/event'
import { useCallback, useEffect, useState } from 'react'
import { cn } from '@/lib/utils'
import {
  deleteLlmModel,
  installLocal,
  localStatus,
  type LocalStatus,
  pullLlmModel,
  setLlmModel,
  setLocal,
} from '../../api/local'

/** 本地千问：开关、4B/8B 下载与选用。 */
export function LocalModel({ onError }: { onError: (message: string) => void }) {
  const [st, setSt] = useState<LocalStatus | null>(null)

  const refresh = useCallback(() => {
    void localStatus()
      .then(setSt)
      .catch(e => onError(String(e)))
  }, [onError])

  useEffect(() => {
    refresh()
    const un = listen<{ message?: string; percent?: number }>('local-model-progress', () => {
      refresh()
    })
    return () => {
      void un.then(f => f())
    }
  }, [refresh])

  if (!st) {
    return null
  }

  const busy = st.busy

  return (
    <div className="flex flex-col gap-2 border-t border-border pt-3">
      <div className="flex items-center justify-between gap-3">
        <span className="text-sm">本地模型</span>
        <button
          type="button"
          disabled={busy}
          aria-label="本地模型"
          className={cn(
            'min-w-[7.5rem] rounded-lg border px-3 py-1.5 text-center text-sm',
            st.enabled
              ? 'border-foreground/40 bg-muted'
              : 'border-border hover:bg-muted/50',
            busy && 'opacity-50',
          )}
          onClick={() => {
            onError('')
            const next = !st.enabled
            void (async () => {
              try {
                if (next && !st.hasModel) {
                  await installLocal()
                } else {
                  await setLocal(next)
                }
                refresh()
              } catch (e) {
                onError(String(e))
                refresh()
              }
            })()
          }}
        >
          {st.enabled ? '开' : '关'}
        </button>
      </div>

      {(st.models ?? []).map(row => {
        const active = row.id === st.model
        return (
          <div key={row.id} className="flex items-center justify-between gap-2">
            <div className="min-w-0 text-sm">
              <span className="font-medium">{row.label}</span>
              <span className="ml-2 text-muted-foreground">{row.id}</span>
            </div>
            <div className="flex shrink-0 gap-1.5">
              {!row.installed ? (
                <button
                  type="button"
                  disabled={busy}
                  className="rounded-lg border border-border px-2.5 py-1 text-sm hover:bg-muted/50 disabled:opacity-50"
                  onClick={() => {
                    onError('')
                    void pullLlmModel(row.id)
                      .then(setSt)
                      .catch(e => {
                        onError(String(e))
                        refresh()
                      })
                  }}
                >
                  下载
                </button>
              ) : active ? (
                <span className="rounded-lg border border-foreground/40 bg-muted px-2.5 py-1 text-sm">
                  使用中
                </span>
              ) : (
                <>
                  <button
                    type="button"
                    disabled={busy}
                    className="rounded-lg border border-border px-2.5 py-1 text-sm hover:bg-muted/50 disabled:opacity-50"
                    onClick={() => {
                      onError('')
                      void setLlmModel(row.id)
                        .then(setSt)
                        .catch(e => {
                          onError(String(e))
                          refresh()
                        })
                    }}
                  >
                    选用
                  </button>
                  <button
                    type="button"
                    disabled={busy}
                    className="rounded-lg border border-border px-2.5 py-1 text-sm hover:bg-muted/50 disabled:opacity-50"
                    onClick={() => {
                      onError('')
                      void deleteLlmModel(row.id)
                        .then(setSt)
                        .catch(e => {
                          onError(String(e))
                          refresh()
                        })
                    }}
                  >
                    删除
                  </button>
                </>
              )}
            </div>
          </div>
        )
      })}

      {(busy || st.message) && (
        <p className="text-xs text-muted-foreground">
          {st.message || '…'}
          {busy && ` ${st.percent}%`}
        </p>
      )}
    </div>
  )
}
