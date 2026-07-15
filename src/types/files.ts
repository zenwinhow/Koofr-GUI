export type FileKind = 'folder' | 'xlsx' | 'pdf' | 'docx' | 'image'

export interface CloudFile {
  id: string
  name: string
  kind: FileKind
  owner: string
  modifiedAt: string
  size: string
}
