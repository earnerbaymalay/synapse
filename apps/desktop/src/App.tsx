import { useState } from 'react'
import { invoke } from '@tauri-apps/api/tauri'

function App() {
  const [version, setVersion] = useState('')

  useState(() => {
    invoke('get_version').then((v) => setVersion(v as string))
  })

  return (
    <div className="p-4">
      <h1 className="text-2xl font-bold">LLM Neurosurgeon</h1>
      <p>Version: {version}</p>
    </div>
  )
}

export default App
