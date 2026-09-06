import { useState } from 'react'
import { DictPack } from './DictPack'
import { LocalModel } from './LocalModel'
import { Shortcuts } from './Shortcuts'

/** 设置页：快捷键 + 本地模型 + 词典。 */
export function Settings() {
  const [error, setError] = useState('')

  return (
    <div className="flex flex-col gap-2 p-4">
      <Shortcuts onError={setError} />
      <LocalModel onError={setError} />
      <DictPack onError={setError} />
      {error && <p className="text-sm text-destructive">{error}</p>}
    </div>
  )
}
