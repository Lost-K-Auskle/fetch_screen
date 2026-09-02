import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { listen, emit } from '@tauri-apps/api/event';

interface ScrollProgress {
  frame_count: number;
  total_height: number;
  max_length: number;
}

interface CaptureRegion {
  x: number; y: number; width: number; height: number;
}

/**
 * 滚动截图浮动工具栏 — 底部居中大面板。
 * 默认手动模式：用户自己滚动页面，系统持续截帧拼接。
 * 支持切换到自动滚动模式。
 */
export default function ScrollCaptureToolbar() {
  const [progress, setProgress] = useState<ScrollProgress | null>(null);
  const [status, setStatus] = useState<'capturing' | 'stopping' | 'done' | 'error'>('capturing');
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [autoMode, setAutoMode] = useState(false);
  const [warning, setWarning] = useState<string | null>(null);

  // 挂载 → 读选区 → 启动手动捕获
  useEffect(() => {
    invoke<CaptureRegion | null>('get_scroll_region')
      .then((region) => {
        if (!region) { setErrorMsg('没有待捕获的选区'); setStatus('error'); return; }
        invoke('start_scroll_capture', {
          x: region.x, y: region.y,
          width: region.width, height: region.height,
          mode: 'manual',
        }).catch((err: any) => { setErrorMsg(String(err)); setStatus('error'); });
      })
      .catch((err) => { setErrorMsg(String(err)); setStatus('error'); });

    return () => { invoke('stop_scroll_capture').catch(() => {}); };
  }, []);

  // 监听进度/完成/错误
  useEffect(() => {
    const uns: Array<() => void> = [];

    listen<ScrollProgress>('scroll:progress', (e) => {
      setProgress(e.payload);
    }).then((fn) => uns.push(fn));

    listen<{ path: string }>('scroll:complete', (e) => {
      setStatus('done');
      emit('scroll:done', { path: e.payload.path }).catch(() => {});
      setTimeout(() => getCurrentWindow().close().catch(() => {}), 600);
    }).then((fn) => uns.push(fn));

    listen<{ message: string }>('scroll:error', (e) => {
      setErrorMsg(e.payload.message);
      setStatus('error');
    }).then((fn) => uns.push(fn));

    // 滚动太快警告：显示提示条，2 秒后自动消失
    listen<{ message: string }>('scroll:warning', (e) => {
      setWarning(e.payload.message);
      setTimeout(() => setWarning(null), 2000);
    }).then((fn) => uns.push(fn));

    return () => { uns.forEach((fn) => fn()); };
  }, []);

  const handleStop = async () => {
    setStatus('stopping');
    try { await invoke('stop_scroll_capture'); }
    catch (err) { console.error(err); }
  };

  const handleSwitchToAuto = async () => {
    setAutoMode(true);
    setProgress(null);
    try {
      // 直接以 auto 模式重启：后端 start_scroll_capture 会取消旧线程并递增 gen，
      // 旧线程的 scroll:complete 被抑制，选区框/工具栏不会被误关。
      const region = await invoke<CaptureRegion | null>('get_scroll_region');
      if (!region) return;
      setStatus('capturing');
      await invoke('start_scroll_capture', {
        x: region.x, y: region.y, width: region.width, height: region.height,
        mode: 'auto',
      });
    } catch (err) {
      setErrorMsg(String(err));
      setStatus('error');
    }
  };

  const handleClose = () => getCurrentWindow().close().catch(() => {});

  // --- 错误状态 ---
  if (status === 'error') {
    return (
      <Panel>
        <span style={{ color: '#ff6b6b', fontSize: 13 }}>⚠️ {errorMsg || '捕获失败'}</span>
        <button onClick={handleClose} style={btnSecondary}>关闭</button>
      </Panel>
    );
  }

  // --- 完成状态 ---
  if (status === 'done') {
    return (
      <Panel>
        <span style={{ color: '#6bff9f', fontSize: 13 }}>✅ 拼接完成，正在预览...</span>
      </Panel>
    );
  }

  // --- 捕获中 ---
  return (
    <Panel>
      {/* 左侧：图标 + 提示 + 统计 */}
      <div style={{ display: 'flex', alignItems: 'center', gap: 10, flex: 1 }}>
        <span style={{ fontSize: 18 }}>📜</span>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
          <span style={{ color: '#ddd', fontSize: 13, fontWeight: 500 }}>
            {autoMode ? '🔽 自动滚动中' : '🖱️ 请手动滚动页面'}
          </span>
          {warning && (
            <span style={{ color: '#ffb84d', fontSize: 12, fontWeight: 600 }}>⚠️ {warning}</span>
          )}
          {progress && (
            <span style={{ color: '#888', fontSize: 11 }}>
              已捕获 {progress.frame_count} 帧 · 高度 {progress.total_height}px
              {progress.total_height >= progress.max_length ? ' (已达上限)' : ''}
            </span>
          )}
        </div>
      </div>

      {/* 进度条 */}
      {progress && (
        <div style={{ width: 60, height: 3, background: '#2a3550', borderRadius: 2, overflow: 'hidden', flexShrink: 0 }}>
          <div style={{
            width: `${Math.min(100, Math.round((progress.total_height / progress.max_length) * 100))}%`,
            height: '100%', background: 'linear-gradient(90deg, #533483, #7b5ea7)',
            borderRadius: 2, transition: 'width 0.2s',
          }} />
        </div>
      )}

      {/* 右侧按钮组 */}
      <div style={{ display: 'flex', gap: 6, flexShrink: 0 }}>
        {!autoMode && status === 'capturing' && (
          <button onClick={handleSwitchToAuto} style={btnSmall} title="切换为自动滚动">
            🔄 自动
          </button>
        )}
        <button
          onClick={handleStop}
          disabled={status === 'stopping'}
          style={{
            ...btnPrimary,
            opacity: status === 'stopping' ? 0.5 : 1,
            cursor: status === 'stopping' ? 'default' : 'pointer',
          }}
        >
          {status === 'stopping' ? '停止中...' : '⏹ 停止截图'}
        </button>
      </div>
    </Panel>
  );
}

/** 底部居中面板外壳 */
function Panel({ children }: { children: React.ReactNode }) {
  const handleMouseDown = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest('button')) return;
    if (e.button === 0) getCurrentWindow().startDragging().catch(() => {});
  };

  return (
    <div
      onMouseDown={handleMouseDown}
      style={{
        width: '100vw', height: '100vh',
        display: 'flex', alignItems: 'flex-end', justifyContent: 'center',
        paddingBottom: 2, background: 'transparent', userSelect: 'none',
      }}
    >
      <div style={{
        display: 'flex', alignItems: 'center', gap: 12,
        background: 'rgba(22, 24, 44, 0.94)',
        border: '1px solid rgba(180,160,220,0.3)',
        borderRadius: 12, padding: '10px 18px',
        backdropFilter: 'blur(16px)',
        boxShadow: '0 4px 24px rgba(0,0,0,0.6)',
        cursor: 'move', minWidth: 380, maxWidth: 500,
      }}>
        {children}
      </div>
    </div>
  );
}

const btnPrimary: React.CSSProperties = {
  padding: '7px 18px', borderRadius: 8,
  border: '1px solid rgba(200,160,255,0.5)',
  background: 'rgba(100,60,170,0.55)',
  color: '#e8dcff', fontSize: 13, fontWeight: 600,
  whiteSpace: 'nowrap', lineHeight: 1.2,
};

const btnSecondary: React.CSSProperties = {
  padding: '6px 14px', borderRadius: 6,
  border: '1px solid rgba(255,255,255,0.15)',
  background: 'rgba(255,255,255,0.08)',
  color: '#ccc', fontSize: 12, cursor: 'pointer',
  whiteSpace: 'nowrap',
};

const btnSmall: React.CSSProperties = {
  padding: '5px 10px', borderRadius: 6,
  border: '1px solid rgba(255,255,255,0.12)',
  background: 'rgba(255,255,255,0.06)',
  color: '#aaa', fontSize: 11, cursor: 'pointer',
  whiteSpace: 'nowrap', lineHeight: 1.2,
};
