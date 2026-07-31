import { useState, useEffect, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import ScreenshotOverlay from './pages/overlay/ScreenshotOverlay';
import { useScreenshotStore } from './stores/screenshotStore';

function App() {
  const store = useScreenshotStore();
  const [mode, setMode] = useState<'home' | 'screenshot_region' | 'screenshot_full' | 'scroll_capture'>('home');

  // 监听全局热键事件
  useEffect(() => {
    const unlisteners: Array<() => void> = [];

    listen('hotkey:screenshot_region', () => {
      startRegionCapture();
    }).then((fn) => { unlisteners.push(fn); });

    listen('hotkey:screenshot_full', () => {
      startFullCapture();
    }).then((fn) => { unlisteners.push(fn); });

    listen('hotkey:scroll_capture', () => {
      setMode('scroll_capture');
    }).then((fn) => { unlisteners.push(fn); });

    listen('tray:screenshot', () => {
      startRegionCapture();
    }).then((fn) => { unlisteners.push(fn); });

    return () => {
      unlisteners.forEach((fn) => fn());
    };
  }, []);

  // 开始区域截图
  const startRegionCapture = useCallback(async () => {
    try {
      // 先全屏截图作为基础
      const path = await invoke<string>('capture_fullscreen');
      store.setFullscreenCachePath(path);
      setMode('screenshot_region');
    } catch (err) {
      console.error('全屏截图失败:', err);
    }
  }, [store]);

  // 开始全屏截图 (直接保存)
  const startFullCapture = useCallback(async () => {
    try {
      const path = await invoke<string>('capture_fullscreen');
      await invoke('copy_to_clipboard', { imagePath: path });
      await invoke('save_to_file', { imagePath: path, destPath: null, format: null });
      store.setLastScreenshotPath(path);
    } catch (err) {
      console.error('全屏截图失败:', err);
    }
  }, [store]);

  // 区域截图完成
  const handleRegionComplete = useCallback(async (regionPath: string) => {
    try {
      await invoke('copy_to_clipboard', { imagePath: regionPath });
      await invoke('save_to_file', { imagePath: regionPath, destPath: null, format: null });
      store.setLastScreenshotPath(regionPath);
      store.setFullscreenCachePath(null);
      setMode('home');
    } catch (err) {
      console.error('保存截图失败:', err);
    }
  }, [store]);

  // 取消截图
  const handleCancel = useCallback(() => {
    store.setFullscreenCachePath(null);
    setMode('home');
  }, [store]);

  // 贴图
  const handlePinImage = useCallback(async () => {
    if (!store.lastScreenshotPath) return;
    try {
      const pinId = await invoke<string>('create_pin_window', {
        imagePath: store.lastScreenshotPath,
        x: 200, y: 200, width: 400, height: 300,
      });
      store.addPin(pinId);
    } catch (err) {
      console.error('贴图失败:', err);
    }
  }, [store]);

  // 渲染截图覆盖层
  if (mode === 'screenshot_region' && store.fullscreenCachePath) {
    return (
      <ScreenshotOverlay
        imagePath={store.fullscreenCachePath}
        onComplete={handleRegionComplete}
        onCancel={handleCancel}
      />
    );
  }

  // 主界面
  return (
    <div style={{
      width: '100vw',
      height: '100vh',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      background: '#1a1a2e',
      color: '#e0e0e0',
      fontFamily: 'system-ui, -apple-system, sans-serif',
    }}>
      <h1 style={{ fontSize: '2.2rem', marginBottom: '0.3rem', fontWeight: 700 }}>
        📷 Fetch Screen
      </h1>
      <p style={{ color: '#888', marginBottom: '2.5rem', fontSize: '0.95rem' }}>
        截图 · 滚动长截图 · 贴图置顶
      </p>

      <div style={{
        background: '#16213e',
        padding: '2rem',
        borderRadius: '16px',
        minWidth: '380px',
        display: 'flex',
        flexDirection: 'column',
        gap: '1rem',
      }}>
        <ActionButton
          icon="🔲"
          label="区域截图"
          hint="Alt+A"
          color="#0f3460"
          onClick={startRegionCapture}
        />
        <ActionButton
          icon="🖥️"
          label="全屏截图"
          hint="Ctrl+Alt+A"
          color="#0f3460"
          onClick={startFullCapture}
        />
        <ActionButton
          icon="📜"
          label="滚动长截图"
          hint="Ctrl+Shift+A"
          color="#533483"
          onClick={() => setMode('scroll_capture')}
        />

        {/* 贴图操作 */}
        {store.lastScreenshotPath && (
          <ActionButton
            icon="📌"
            label="贴图置顶"
            hint="Ctrl+T"
            color="#1a5c3a"
            onClick={handlePinImage}
          />
        )}

        {/* 贴图数量指示 */}
        {store.pinIds.length > 0 && (
          <p style={{ textAlign: 'center', color: '#666', fontSize: '0.85rem', margin: 0 }}>
            📌 {store.pinIds.length} 张贴图活跃中 · 双击贴图切换交互模式
          </p>
        )}
      </div>

      {/* 状态栏 */}
      <div style={{ marginTop: '1.5rem', color: '#555', fontSize: '0.8rem' }}>
        {store.lastScreenshotPath ? '最近截图已就绪' : '按下热键或点击按钮开始截图'}
      </div>
    </div>
  );
}

function ActionButton({ icon, label, hint, color, onClick }: {
  icon: string;
  label: string;
  hint: string;
  color: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      style={{
        display: 'flex',
        alignItems: 'center',
        gap: '12px',
        width: '100%',
        padding: '14px 18px',
        background: color,
        color: '#fff',
        border: 'none',
        borderRadius: '10px',
        cursor: 'pointer',
        fontSize: '1rem',
        transition: 'transform 0.1s, opacity 0.1s',
      }}
      onMouseEnter={(e) => { e.currentTarget.style.opacity = '0.9'; e.currentTarget.style.transform = 'scale(1.02)'; }}
      onMouseLeave={(e) => { e.currentTarget.style.opacity = '1'; e.currentTarget.style.transform = 'scale(1)'; }}
    >
      <span style={{ fontSize: '1.4rem' }}>{icon}</span>
      <span style={{ flex: 1, textAlign: 'left' }}>{label}</span>
      <span style={{ fontSize: '0.8rem', opacity: 0.7, fontFamily: 'monospace' }}>{hint}</span>
    </button>
  );
}

export default App;
