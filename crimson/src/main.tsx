import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import App from './App.tsx'
import LyricsApp from './LyricsApp.tsx'
import { LCUProvider } from './contexts/LCUContext.tsx'
import { AuthProvider } from './contexts/AuthContext.tsx'

const isLyricsWindow = window.location.search.includes('window=lyrics');

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <AuthProvider>
      <LCUProvider>
          {isLyricsWindow ? <LyricsApp /> : <App />}
      </LCUProvider>
    </AuthProvider>
  </StrictMode>,
)
