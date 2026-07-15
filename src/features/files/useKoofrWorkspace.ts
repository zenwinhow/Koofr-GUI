import { useCallback, useEffect, useRef, useState } from 'react'
import { commandErrorMessage, koofr } from '../../services/koofr'
import type { KoofrMount, RemoteFile } from '../../types/backend'

type WorkspaceStatus = 'idle' | 'loading' | 'ready' | 'error'

interface WorkspaceState {
  mounts: KoofrMount[]
  activeMountId: string
  path: string
  files: RemoteFile[]
  status: WorkspaceStatus
  error: string
  lastSyncedAt: Date | null
}

const initialState: WorkspaceState = {
  mounts: [],
  activeMountId: '',
  path: '/',
  files: [],
  status: 'idle',
  error: '',
  lastSyncedAt: null,
}

function chooseInitialMount(mounts: KoofrMount[]) {
  return mounts.find((mount) => mount.isPrimary && mount.online)
    ?? mounts.find((mount) => mount.online)
    ?? mounts.find((mount) => mount.isPrimary)
    ?? mounts[0]
}

export function useKoofrWorkspace(enabled: boolean) {
  const [state, setState] = useState<WorkspaceState>(initialState)
  const requestSequence = useRef(0)

  const initialize = useCallback(async () => {
    const requestId = ++requestSequence.current
    setState((current) => ({ ...current, status: 'loading', error: '' }))

    try {
      const mounts = await koofr.listMounts()
      const activeMount = chooseInitialMount(mounts)
      if (!activeMount) {
        if (requestId === requestSequence.current) {
          setState({ ...initialState, status: 'error', error: '账户中没有可访问的存储位置。' })
        }
        return
      }

      const files = await koofr.listFiles(activeMount.id, '/')
      if (requestId !== requestSequence.current) return
      setState({
        mounts,
        activeMountId: activeMount.id,
        path: '/',
        files,
        status: 'ready',
        error: '',
        lastSyncedAt: new Date(),
      })
    } catch (error) {
      if (requestId !== requestSequence.current) return
      setState((current) => ({
        ...current,
        status: 'error',
        error: commandErrorMessage(error, '无法读取 Koofr 文件，请稍后重试。'),
      }))
    }
  }, [])

  const loadDirectory = useCallback(async (mountId: string, path: string) => {
    const requestId = ++requestSequence.current
    setState((current) => ({
      ...current,
      activeMountId: mountId,
      path,
      files: [],
      status: 'loading',
      error: '',
    }))

    try {
      const files = await koofr.listFiles(mountId, path)
      if (requestId !== requestSequence.current) return
      setState((current) => ({
        ...current,
        activeMountId: mountId,
        path,
        files,
        status: 'ready',
        error: '',
        lastSyncedAt: new Date(),
      }))
    } catch (error) {
      if (requestId !== requestSequence.current) return
      setState((current) => ({
        ...current,
        status: 'error',
        error: commandErrorMessage(error, '无法读取这个文件夹，请稍后重试。'),
      }))
    }
  }, [])

  const refresh = useCallback(() => {
    if (!state.activeMountId) return Promise.resolve()
    return loadDirectory(state.activeMountId, state.path)
  }, [loadDirectory, state.activeMountId, state.path])

  useEffect(() => {
    if (enabled) {
      void initialize()
    } else {
      requestSequence.current += 1
      setState(initialState)
    }
  }, [enabled, initialize])

  return {
    ...state,
    initialize,
    loadDirectory,
    refresh,
  }
}
