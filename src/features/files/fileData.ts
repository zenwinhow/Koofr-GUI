import type { CloudFile } from '../../types/files'

export const initialFiles: CloudFile[] = [
  { id: 'design', name: '设计资料', kind: 'folder', owner: '我', modifiedAt: '2024-05-15 10:24', size: '—' },
  { id: 'screens', name: '产品截图', kind: 'folder', owner: '我', modifiedAt: '2024-05-14 16:08', size: '—' },
  { id: 'travel', name: '旅行照片', kind: 'folder', owner: '我', modifiedAt: '2024-05-12 09:31', size: '—' },
  { id: 'budget', name: 'Q3 预算.xlsx', kind: 'xlsx', owner: '我', modifiedAt: '2024-05-15 09:12', size: '238 KB' },
  { id: 'brand', name: '品牌指南.pdf', kind: 'pdf', owner: '我', modifiedAt: '2024-05-14 11:47', size: '3.2 MB' },
  { id: 'notes', name: '会议记录.docx', kind: 'docx', owner: '我', modifiedAt: '2024-05-13 15:36', size: '156 KB' },
]
