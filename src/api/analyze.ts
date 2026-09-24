import type { Analysis } from '../atoms'
import { invoke } from '@tauri-apps/api/core'

export function analyze(
  text: string,
  into?: string,
  select?: boolean,
  reqId?: number,
  native?: boolean,
) {
  return invoke<Analysis>('analyze', { text, into, select, reqId, native })
}

export function sentenceGrammar(sentence: string) {
  return invoke<string>('sentence_grammar', { sentence })
}

export function grammarAsk(sentence: string, grammar: string, question: string) {
  return invoke<string>('grammar_ask', { sentence, grammar, question })
}
