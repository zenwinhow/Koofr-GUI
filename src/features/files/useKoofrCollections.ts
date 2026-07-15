import { useCallback, useEffect, useRef, useState } from 'react'
import { commandErrorDiagnostic, commandErrorMessage, koofr } from '../../services/koofr'
import type { LocatedFile, TrashItem } from '../../types/backend'

export type CollectionView = '最近的文件' | '已共享' | '回收站'
type CollectionStatus = 'idle' | 'loading' | 'ready' | 'error'

interface CollectionState {
  view: CollectionView | null
  files: LocatedFile[]
  trash: TrashItem[]
  retentionDays: number
  status: CollectionStatus
  error: string
  diagnostic: string
  lastSyncedAt: Date | null
}

const initialState: CollectionState = {
  view: null,
  files: [],
  trash: [],
  retentionDays: 0,
  status: 'idle',
  error: '',
  diagnostic: '',
  lastSyncedAt: null,
}

export function isCollectionView(value: string): value is CollectionView {
  return value === '最近的文件' || value === '已共享' || value === '回收站'
}

export function useKoofrCollections(enabled: boolean, activeItem: string) {
  const [state, setState] = useState<CollectionState>(initialState)
  const requestSequence = useRef(0)

  const load = useCallback(async (view: CollectionView, forceRefresh = false) => {
    const requestId = ++requestSequence.current
    setState((current) => ({
      ...current,
      view,
      files: [],
      trash: [],
      status: 'loading',
      error: '',
      diagnostic: '',
    }))

    try {
      if (view === '回收站') {
        const trash = await koofr.listTrash(forceRefresh)
        if (requestId !== requestSequence.current) return
        setState({
          view,
          files: [],
          trash: trash.items,
          retentionDays: trash.retentionDays,
          status: 'ready',
          error: '',
          diagnostic: '',
          lastSyncedAt: new Date(),
        })
        return
      }

      const files = await (view === '最近的文件'
        ? koofr.listRecent(forceRefresh)
        : koofr.listShared(forceRefresh))
      if (requestId !== requestSequence.current) return
      setState({
        view,
        files,
        trash: [],
        retentionDays: 0,
        status: 'ready',
        error: '',
        diagnostic: '',
        lastSyncedAt: new Date(),
      })
    } catch (error) {
      if (requestId !== requestSequence.current) return
      setState((current) => ({
        ...current,
        status: 'error',
        error: commandErrorMessage(error, `无法读取${view}内容，请稍后重试。`),
        diagnostic: commandErrorDiagnostic(error),
      }))
    }
  }, [])

  useEffect(() => {
    if (!enabled) {
      requestSequence.current += 1
      setState(initialState)
      return
    }
    if (isCollectionView(activeItem)) void load(activeItem)
  }, [activeItem, enabled, load])

  return {
    ...state,
    load,
    refresh: () => state.view ? load(state.view, true) : Promise.resolve(),
  }
}
