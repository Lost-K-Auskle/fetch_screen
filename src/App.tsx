import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useScreenshotStore } from './stores/screenshotStore';
import SettingsModal, { AppConfig } from './components/SettingsModal';

type OverlayIntent = 'region' | 'scroll';

interface RegionCompletePayload {
  path: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

// —— 简约主题：单一中性色板 + 一个克制的主色，hover 才点亮 ——
const palette = {
  bg: '#0f1116',
  surface: '#171a21',
  surfaceHover: '#1b1f28',
  border: '#252a34',
  accent: '#4f7cff',
  text: '#e8eaf0',
  textDim: '#9aa1ad',
  textFaint: '#6b7280',
};

const css = `
  .fs-btn {
    display: flex; align-items: center; justify-content: space-between;
    width: 100%; padding: 13px 16px;
    background: ${palette.surface}; border: 1px solid ${palette.border}; border-radius: 10px;
    color: ${palette.text}; cursor: pointer; font-size: 14px;
    transition: border-color .15s ease, background .15s ease;
  }
  .fs-btn:hover { border-color: ${palette.accent}; background: ${palette.surfaceHover}; }
  .fs-btn:active { background: #12151c; }
  .fs-ghost {
    background: transparent; border: none; color: ${palette.textDim};
    font-size: 13px; cursor: pointer; padding: 6px 12px; border-radius: 7px;
    transition: color .15s ease, background .15s ease;
  }
  .fs-ghost:hover { color: ${palette.text}; background: ${palette.surface}; }
`;

function App() {
  const store = useScreenshotStore();
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  // 标记 overlay 的用途：普通区域截图 / 滚动截图选区
  const overlayIntent = useRef<OverlayIntent>('region');

  // 成功提示自动消失
  useEffect(() => {
    if (!success) return;
    const t = setTimeout(() => setSuccess(null), 4000);
    return () => clearTimeout(t);
  }, [success]);

  // 加载配置（按钮快捷键提示用）
  useEffect(() => {
    invoke('get_config')
      .then((c) => setAppConfig(c as AppConfig))
      .catch(console.error);
  }, []);

  // 监听全局热键事件
  useEffect(() => {
    let cancelled = false;
    const unlisteners: Array<() => void> = [];

    const add = (event: string, handler: (event: any) => void) => {
      listen(event, handler).then((un) => {
        if (cancelled) un();
        else unlisteners.push(un);
      });
    };

    add('hotkey:screenshot_region', () => {
      overlayIntent.current = 'region';
      startRegionCapture();
    });
    add('hotkey:screenshot_full', () => startFullCapture());
    add('hotkey:scroll_capture', () => {
      overlayIntent.current = 'scroll';
      startRegionCapture(); // 先框选区域
    });
    add('tray:screenshot', () => {
      overlayIntent.current = 'region';
      startRegionCapture();
    });

    // 全屏选区覆盖层的结果
    add('region:complete', (event) => {
      const payload = event.payload as RegionCompletePayload;
      if (overlayIntent.current === 'scroll') {
        handleScrollRegionConfirmed(payload);
      } else {
        handleRegionComplete(payload.path);
      }
      overlayIntent.current = 'region'; // 重置
    });
    add('region:cancelled', () => {
      overlayIntent.current = 'region';
      getCurrentWindow().show().catch(() => {});
    });

    // 滚动截图完成 → 显示预览
    add('scroll:done', (event) => {
      const { path } = event.payload as { path: string };
      getCurrentWindow().show().catch(() => {});
      invoke('show_preview', { imagePath: path }).catch((err) => {
        console.error('显示预览失败:', err);
        setError(`显示预览失败: ${err}`);
      });
      setSuccess('滚动长截图完成');
      // 保存到图片目录（剪贴板已由后端自动复制）
      invoke('save_to_file', { imagePath: path, destPath: null, format: null }).catch(() => {});
    });

    return () => {
      cancelled = true;
      unlisteners.forEach((fn) => fn());
    };
  }, []);

  // 开始区域截图：打开全屏选区覆盖层
  const startRegionCapture = useCallback(async () => {
    try {
      await invoke('open_region_overlay');
      setError(null);
    } catch (err) {
      console.error('打开选区覆盖层失败:', err);
      setError(`打开选区覆盖层失败: ${err}`);
    }
  }, []);

  // 开始全屏截图 (直接保存)
  const startFullCapture = useCallback(async () => {
    try {
      const path = await invoke<string>('capture_fullscreen');
      store.setLastScreenshotPath(path);
      setError(null);
      setSuccess('已截图，请操作浮窗');
      invoke('show_preview', { imagePath: path }).catch((err) => {
        console.error('显示预览失败:', err);
        setError(`显示预览失败: ${err}`);
      });
      // 剪贴板已由后端自动复制
      invoke('save_to_file', { imagePath: path, destPath: null, format: null })
        .catch((err) => { console.error('保存失败:', err); setError(`保存失败: ${err}`); });
    } catch (err) {
      console.error('全屏截图失败:', err);
      setError(`全屏截图失败: ${err}`);
    }
  }, [store]);

  // 区域截图完成（来自全屏选区覆盖层）
  const handleRegionComplete = useCallback(async (regionPath: string) => {
    store.setLastScreenshotPath(regionPath);
    setError(null);
    setSuccess('区域截图完成');
    invoke('show_preview', { imagePath: regionPath }).catch((err) => {
      console.error('显示预览失败:', err);
      setError(`显示预览失败: ${err}`);
    });
    // 剪贴板已由后端自动复制
    invoke('save_to_file', { imagePath: regionPath, destPath: null, format: null })
      .catch((err) => { console.error('保存失败:', err); setError(`保存失败: ${err}`); });
    try { await getCurrentWindow().show(); } catch { /* ignore */ }
  }, [store]);

  // 滚动截图选区确认 → 隐藏主窗口 → 开进度窗口 → 启动后端捕获
  const handleScrollRegionConfirmed = useCallback(async (payload: RegionCompletePayload) => {
    setError(null);
    // 隐藏主窗口（避免出现在后续截帧中）；除非用户在设置里关闭了"截图时隐藏 UI"
    try {
      const cfg = await invoke<AppConfig>('get_config');
      if (cfg?.hide_ui_on_capture !== false) {
        await getCurrentWindow().hide();
      }
    } catch { /* ignore */ }

    // 打开选区边框浮层（点击穿透，边滚边看选区）
    try {
      await invoke('open_scroll_region_frame', {
        x: payload.x,
        y: payload.y,
        width: payload.width,
        height: payload.height,
      });
    } catch (err) {
      console.error('打开选区边框失败:', err);
    }

    // 打开进度浮层
    try {
      await invoke('open_scroll_toolbar', {
        x: payload.x,
        y: payload.y,
        width: payload.width,
        height: payload.height,
      });
    } catch (err) {
      console.error('打开进度窗口失败:', err);
      setError(`打开进度窗口失败: ${err}`);
      try { await getCurrentWindow().show(); } catch { /* ignore */ }
    }
  }, []);

  const openCache = () => invoke('open_cache_dir').catch((err) => setError(`打开缓存目录失败: ${err}`));

  return (
    <>
      <style>{css}</style>
      <div style={{
        height: '100vh', display: 'flex', flexDirection: 'column',
        background: palette.bg, color: palette.text,
        fontFamily: "system-ui, -apple-system, 'Segoe UI', sans-serif",
      }}>
        {/* 顶栏：应用名 + 功能按钮 */}
        <header style={{
          display: 'flex', alignItems: 'center', justifyContent: 'space-between',
          padding: '14px 18px', borderBottom: `1px solid ${palette.border}`,
        }}>
          <div style={{ fontSize: 15, fontWeight: 600, letterSpacing: '0.02em', color: palette.text }}>
            Fetch Screen
          </div>
          <div style={{ display: 'flex', gap: 4 }}>
            <button className="fs-ghost" onClick={openCache} title="打开截图缓存目录">缓存目录</button>
            <button className="fs-ghost" onClick={() => setSettingsOpen(true)} title="设置">设置</button>
          </div>
        </header>

        {/* 主体：标题 + 动作 */}
        <main style={{
          flex: 1, display: 'flex', flexDirection: 'column',
          alignItems: 'center', justifyContent: 'center', padding: '24px',
        }}>
          <div style={{ fontSize: 26, fontWeight: 600, marginBottom: 6, color: palette.text }}>截取屏幕</div>
          <div style={{ fontSize: 13, color: palette.textFaint, marginBottom: 30 }}>区域截图 · 全屏截图 · 滚动长截图</div>

          <div style={{ display: 'flex', flexDirection: 'column', gap: 10, width: '100%', maxWidth: 340 }}>
            <button className="fs-btn" onClick={() => { overlayIntent.current = 'region'; startRegionCapture(); }}>
              <span>区域截图</span>
              <span style={{ fontSize: 12, color: palette.textDim, fontFamily: 'ui-monospace, monospace' }}>
                {appConfig?.hotkeys.screenshot ?? 'Alt+Shift+A'}
              </span>
            </button>
            <button className="fs-btn" onClick={startFullCapture}>
              <span>全屏截图</span>
              <span style={{ fontSize: 12, color: palette.textDim, fontFamily: 'ui-monospace, monospace' }}>
                {appConfig?.hotkeys.screenshot_full ?? 'Ctrl+Alt+A'}
              </span>
            </button>
            <button className="fs-btn" onClick={() => { overlayIntent.current = 'scroll'; startRegionCapture(); }}>
              <span>滚动长截图</span>
              <span style={{ fontSize: 12, color: palette.textDim, fontFamily: 'ui-monospace, monospace' }}>
                {appConfig?.hotkeys.scrollshot ?? 'Ctrl+Shift+A'}
              </span>
            </button>
          </div>
        </main>

        {/* 底部状态栏 */}
        <footer style={{
          padding: '12px 18px', borderTop: `1px solid ${palette.border}`,
          fontSize: 12.5, textAlign: 'center', color: palette.textFaint,
          minHeight: 44, display: 'flex', alignItems: 'center', justifyContent: 'center',
        }}>
          {error ? <span style={{ color: '#f87171' }}>{error}</span>
            : success ? <span style={{ color: '#6ee7a0' }}>{success}</span>
            : <span>{store.lastScreenshotPath ? '最近截图已就绪' : '按热键或点击按钮开始截图'}</span>}
        </footer>

        {/* 设置弹窗 */}
        {settingsOpen && appConfig && (
          <SettingsModal
            config={appConfig}
            onSave={(c) => setAppConfig(c)}
            onClose={() => setSettingsOpen(false)}
          />
        )}
      </div>
    </>
  );
}

export default App;
