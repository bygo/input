import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { Root } from './pages/Root'
import './index.css'

{
  const mq = window.matchMedia('(prefers-color-scheme: dark)')
  const apply = () => document.documentElement.classList.toggle('dark', mq.matches)
  apply()
  mq.addEventListener('change', apply)
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Root />
  </StrictMode>,
)
