import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'

import '../../../src/styles.css'
import { SplitUploadDialog } from '../../../src/features/transfers/SplitUploadDialog'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <div className="app-shell">
      <SplitUploadDialog
        fileName="2026 年家庭旅行原始视频.mkv"
        onClose={() => undefined}
        onConfirm={() => undefined}
      />
    </div>
  </StrictMode>,
)
