import type { DictStatus } from '../../api/dict'
import { listen } from '@tauri-apps/api/event'
import { useCallback, useEffect, useState } from 'react'
import {
  deleteDictPack,
  dictStatus,
  pullDictPack,
  setDictPack,
} from '../../api/dict'

/** 词库：精简内置 / 完整 ECDICT 下载与选用。 */
export function DictPack({ onError }: { onError: (message: string) => void }) {
  const [st, setSt] = useState<DictStatus | null>(null)

  const refresh = useCallback(() => {
    void dictStatus()
      .then(setSt)
      .catch(e => onError(String(e)))
  }, [onError])

  useEffect(() => {
    refresh()
    const un = listen<{ message?: string, percent?: number }>('dict-progress', () => {
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
      <span className="text-sm">词典</span>

      {st.packs.map((row) => {
        const active = row.id === st.pack
        return (
          <div key={row.id} className="flex items-center justify-between gap-2">
            <div className="min-w-0 text-sm">
              <span className="font-medium">{row.label}</span>
              <span className="ml-2 text-muted-foreground">{row.detail}</span>
            </div>
            <div className="flex shrink-0 gap-1.5">
              {!row.installed
                ? (
                    <button
                      type="button"
                      disabled={busy}
                      className="rounded-lg border border-border px-2.5 py-1 text-sm hover:bg-muted/50 disabled:opacity-50"
                      onClick={() => {
                        onError('')
                        void pullDictPack(row.id)
                          .then(setSt)
                          .catch((e) => {
                            onError(String(e))
                            refresh()
                          })
                      }}
                    >
                      下载
                    </button>
                  )
                : active
                  ? (
                      <span className="rounded-lg border border-foreground/40 bg-muted px-2.5 py-1 text-sm">
                        使用中
                      </span>
                    )
                  : (
                      <>
                        <button
                          type="button"
                          disabled={busy}
                          className="rounded-lg border border-border px-2.5 py-1 text-sm hover:bg-muted/50 disabled:opacity-50"
                          onClick={() => {
                            onError('')
                            void setDictPack(row.id)
                              .then(setSt)
                              .catch((e) => {
                                onError(String(e))
                                refresh()
                              })
                          }}
                        >
                          选用
                        </button>
                        {row.id === 'full' && (
                          <button
                            type="button"
                            disabled={busy}
                            className="rounded-lg border border-border px-2.5 py-1 text-sm hover:bg-muted/50 disabled:opacity-50"
                            onClick={() => {
                              onError('')
                              void deleteDictPack(row.id)
                                .then(setSt)
                                .catch((e) => {
                                  onError(String(e))
                                  refresh()
                                })
                            }}
                          >
                            删除
                          </button>
                        )}
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
