import '@testing-library/jest-dom/vitest'
import { cleanup } from '@testing-library/react'
import { afterEach } from 'vitest'

// Testing Library only self-registers its cleanup when vitest runs with
// `globals: true`, which this project does not. Without it, every render in a
// file accumulates in the same document and queries match across tests.
afterEach(cleanup)
