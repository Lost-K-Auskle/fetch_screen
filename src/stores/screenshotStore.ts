import { create } from 'zustand';

export type ScreenshotAction = 'idle' | 'selecting' | 'annotating' | 'pinning' | 'scrolling';

interface ScreenshotState {
  // 当前动作状态
  action: ScreenshotAction;
  setAction: (action: ScreenshotAction) => void;

  // 最近一次截图路径
  lastScreenshotPath: string | null;
  setLastScreenshotPath: (path: string | null) => void;

  // 全屏截图缓存 (用于选区)
  fullscreenCachePath: string | null;
  setFullscreenCachePath: (path: string | null) => void;

  // 贴图列表
  pinIds: string[];
  addPin: (id: string) => void;
  removePin: (id: string) => void;

  // 设置面板可见性
  settingsOpen: boolean;
  setSettingsOpen: (open: boolean) => void;
}

export const useScreenshotStore = create<ScreenshotState>((set) => ({
  action: 'idle',
  setAction: (action) => set({ action }),

  lastScreenshotPath: null,
  setLastScreenshotPath: (path) => set({ lastScreenshotPath: path }),

  fullscreenCachePath: null,
  setFullscreenCachePath: (path) => set({ fullscreenCachePath: path }),

  pinIds: [],
  addPin: (id) => set((state) => ({ pinIds: [...state.pinIds, id] })),
  removePin: (id) => set((state) => ({ pinIds: state.pinIds.filter((p) => p !== id) })),

  settingsOpen: false,
  setSettingsOpen: (open) => set({ settingsOpen: open }),
}));
