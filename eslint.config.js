import antfu from '@antfu/eslint-config'

export default antfu(
  {
    react: true,
    stylistic: {
      indent: 2,
      quotes: 'single',
      braceStyle: '1tbs',
    },
    ignores: [
      'dist/**',
      'src-tauri/**',
      'eval-rewrite/**',
    ],
  },
  {
    rules: {
      // —— 项目自定义：一律花括号，且禁止单行 { return } ——
      'antfu/curly': 'off',
      curly: ['error', 'all'],
      'style/brace-style': ['error', '1tbs', { allowSingleLine: false }],

      // —— 与现有代码对齐：关掉误伤规则 ——
      // Tauri 跟窗 / OCR 需要同步刷 DOM 再量尺寸
      'react/dom-no-flush-sync': 'off',
      // shadcn 组件常连带导出 variants
      'react-refresh/only-export-components': 'off',
      // 词级 diff / 拆词渲染用 index key 足够
      'react/no-array-index-key': 'off',
      // req / session / finishing 等非 DOM ref
      'react/naming-convention-ref-name': 'off',
      // 刻意的挂载焦点、进度订阅
      'react/exhaustive-deps': 'off',
      // 词卡栏高锁定
      'react/set-state-in-effect': 'off',
    },
  },
)
