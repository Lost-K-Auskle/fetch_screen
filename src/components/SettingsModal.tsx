import { useState, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';

export interface HotkeyConfig {
  screenshot: string;
  screenshot_full: string;
  scrollshot: string;
  pin_last: string;
}

/** 与后端 AppConfig 对应的完整结构（仅用于回传，实际来自 get_config） */
export interface AppConfig {
  hotkeys: HotkeyConfig;
  save: { path: string; format: string; quality: number; naming: string };
  pin: { default_opacity: number; default_click_through: boolean; max_pin_count: number; restore_on_startup: boolean };
  scrollshot: { mode: string; max_length: number; scroll_delay_ms: number; jpeg_quality: number };
  annotation: { default_color: string; default_line_width: number; font_size: number };
  hide_ui_on_capture: boolean;
  preview_drag_mode: string;
  preview_bg_mode: string;
}

// 注：pin_last 尚未接线到任何动作，暂不展示
const FIELDS: { key: keyof HotkeyConfig; label: string }[] = [
  { key: 'screenshot', label: '区域截图' },
  { key: 'screenshot_full', label: '全屏截图' },
  { key: 'scrollshot', label: '滚动长截图' },
];

// e.code（物理键名）→ 后端 parse_key 接受的 token
function mapKeyFromCode(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3); // KeyA -> A
  if (/^Digit[0-9]$/.test(code)) return code.slice(5); // Digit1 -> 1
  if (/^F([1-9]|1[0-9]|2[0-4])$/.test(code)) return code; // F1~F24
  const direct: Record<string, string> = {
    Space: 'Space', Enter: 'Enter', Tab: 'Tab', Delete: 'Delete', Backspace: 'Backspace',
    ArrowUp: 'ArrowUp', ArrowDown: 'ArrowDown', ArrowLeft: 'ArrowLeft', ArrowRight: 'ArrowRight',
    Period: 'Period', Minus: 'Minus', Equal: 'Equal', Comma: 'Comma', Slash: 'Slash',
    Semicolon: 'Semicolon', Quote: 'Quote', BracketLeft: 'BracketLeft', BracketRight: 'BracketRight',
    Backslash: 'Backslash', Backquote: 'Backquote', Home: 'Home', End: 'End', Insert: 'Insert',
    PageUp: 'PageUp', PageDown: 'PageDown', PrintScreen: 'PrintScreen', CapsLock: 'CapsLock',
  };
  return direct[code] ?? null;
}

function captureCombo(e: KeyboardEvent): string | null {
  const mods: string[] = [];
  if (e.ctrlKey) mods.push('Ctrl');
  if (e.altKey) mods.push('Alt');
  if (e.shiftKey) mods.push('Shift');
  if (e.metaKey) mods.push('Super');
  if (mods.length === 0) return null; // 必须带修饰键
  const key = mapKeyFromCode(e.code);
  if (!key) return null;
  return [...mods, key].join('+');
}

interface Props {
  config: AppConfig;
  onSave: (config: AppConfig) => void;
  onClose: () => void;
}

export default function SettingsModal({ config, onSave, onClose }: Props) {
  const [values, setValues] = useState<HotkeyConfig>(config.hotkeys);
  const [recording, setRecording] = useState<keyof HotkeyConfig | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [hideUi, setHideUi] = useState<boolean>(config.hide_ui_on_capture ?? true);
  const [dragMode, setDragMode] = useState<string>(config.preview_drag_mode ?? 'left_drag');
  const [bgMode, setBgMode] = useState<string>(config.preview_bg_mode ?? 'black');

  // 打开设置时临时禁用全局热键，避免录制时触发当前快捷键
  useEffect(() => {
    invoke('set_hotkeys_enabled', { enabled: false }).catch(console.error);
    return () => {
      invoke('set_hotkeys_enabled', { enabled: true }).catch(console.error);
    };
  }, []);

  // 录制快捷键
  useEffect(() => {
    if (!recording) return;
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      if (e.key === 'Escape') { setRecording(null); return; }
      if (e.repeat) return;
      const combo = captureCombo(e);
      if (!combo) return;
      setValues((v) => ({ ...v, [recording]: combo }));
      setRecording(null);
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [recording]);

  const handleSave = async () => {
    setSaving(true);
    setError(null);
    try {
      // 先重新拉取磁盘上的最新配置，再合并本次修改。
      // 否则会用挂载时的旧配置覆盖运行期间磁盘上的新值（如 hide_ui_on_capture 被外部改回 true）。
      const latest = await invoke<AppConfig>('get_config');
      const next: AppConfig = { ...latest, hotkeys: values, hide_ui_on_capture: hideUi, preview_drag_mode: dragMode, preview_bg_mode: bgMode };
      await invoke('update_hotkeys', { config: next });
      onSave(next);
      onClose();
    } catch (err) {
      setError(`保存失败：${err}`);
      setSaving(false);
    }
  };

  return (
    <div style={{
      position: 'fixed', inset: 0, background: 'rgba(0,0,0,0.6)',
      display: 'flex', alignItems: 'center', justifyContent: 'center', zIndex: 100,
    }}>
      <div style={{
        background: '#1e2333', borderRadius: 12, padding: '20px 24px', width: 420,
        color: '#e0e0e0', fontFamily: 'system-ui, sans-serif',
      }}>
        <h2 style={{ margin: '0 0 16px', fontSize: '1.2rem' }}>设置</h2>
        <p style={{ margin: '0 0 12px', fontSize: '0.8rem', color: '#888' }}>
          点击某一行，然后按下新的组合键（必须含 Ctrl/Alt/Shift）。Esc 取消录制。
        </p>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          {FIELDS.map(({ key, label }) => (
            <div key={key} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 12 }}>
              <span style={{ fontSize: '0.9rem' }}>{label}</span>
              <button
                onClick={() => setRecording((r) => (r === key ? null : key))}
                style={{
                  minWidth: 160, padding: '7px 12px',
                  background: recording === key ? '#3a4a7a' : '#2a3040',
                  border: recording === key ? '1px solid #5b8cff' : '1px solid #3a4150',
                  borderRadius: 6, color: recording === key ? '#9fc3ff' : '#e0e0e0',
                  fontSize: '0.85rem', fontFamily: 'monospace', cursor: 'pointer',
                }}
              >
                {recording === key ? '请按新快捷键...' : (values[key] || '未设置')}
              </button>
            </div>
          ))}
        </div>

        {/* 常规选项 */}
        <div style={{
          marginTop: 16, paddingTop: 12, borderTop: '1px solid #333',
          display: 'flex', alignItems: 'center', gap: 8,
        }}>
          <input
            type="checkbox"
            id="hideUi"
            checked={hideUi}
            onChange={(e) => setHideUi(e.target.checked)}
            style={{ width: 16, height: 16, accentColor: '#5b8cff', cursor: 'pointer', margin: 0 }}
          />
          <label htmlFor="hideUi" style={{ fontSize: '0.85rem', cursor: 'pointer' }}>
            截图时隐藏 Fetch Screen 主窗口（避免被截进截图）
          </label>
        </div>

        {/* 预览窗鼠标拖拽方式 */}
        <div style={{
          marginTop: 12, paddingTop: 12, borderTop: '1px solid #333',
          display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 8,
        }}>
          <span style={{ fontSize: '0.85rem' }}>预览窗拖拽方式</span>
          <select
            value={dragMode}
            onChange={(e) => setDragMode(e.target.value)}
            style={{
              padding: '6px 10px', background: '#2a3040', border: '1px solid #3a4150',
              borderRadius: 6, color: '#e0e0e0', fontSize: '0.8rem', cursor: 'pointer',
            }}
          >
            <option value="left_drag">左键拖窗 · Shift+左键平移</option>
            <option value="shift_drag">左键平移 · Shift+左键拖窗</option>
          </select>
        </div>

        {/* 预览窗背景（无截图区域） */}
        <div style={{
          marginTop: 12, paddingTop: 12, borderTop: '1px solid #333',
          display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 8,
        }}>
          <span style={{ fontSize: '0.85rem' }}>预览窗背景</span>
          <select
            value={bgMode}
            onChange={(e) => setBgMode(e.target.value)}
            style={{
              padding: '6px 10px', background: '#2a3040', border: '1px solid #3a4150',
              borderRadius: 6, color: '#e0e0e0', fontSize: '0.8rem', cursor: 'pointer',
            }}
          >
            <option value="black">黑色</option>
            <option value="white">白色</option>
            <option value="hollow">镂空（透明）</option>
          </select>
        </div>

        {error && <p style={{ color: '#ff6b6b', fontSize: '0.8rem', marginTop: 12 }}>⚠️ {error}</p>}

        <div style={{ display: 'flex', gap: 10, marginTop: 18, justifyContent: 'flex-end' }}>
          <button
            onClick={() => { invoke('set_hotkeys_enabled', { enabled: true }).catch(() => {}); onClose(); }}
            style={{ padding: '8px 18px', background: '#333', border: 'none', borderRadius: 6, color: '#ddd', cursor: 'pointer' }}
          >取消</button>
          <button
            onClick={handleSave}
            disabled={saving}
            style={{ padding: '8px 18px', background: '#0f3460', border: 'none', borderRadius: 6, color: '#fff', cursor: 'pointer' }}
          >{saving ? '保存中...' : '保存'}</button>
        </div>
      </div>
    </div>
  );
}
