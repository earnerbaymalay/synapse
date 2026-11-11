import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'

// App mounts its own HintsProvider, so tests and this entry render the same tree.
const root = document.getElementById('root')
if (!root) throw new Error('synapse: #root element missing from index.html')

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
