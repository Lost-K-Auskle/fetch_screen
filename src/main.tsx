import ReactDOM from 'react-dom/client';
import App from './App';
import PinWindow from './pages/pin/PinWindow';
import FullscreenOverlay from './pages/overlay/FullscreenOverlay';
import PreviewWindow from './pages/preview/PreviewWindow';
import ScrollCaptureToolbar from './pages/overlay/ScrollCaptureToolbar';
import ScrollRegionFrame from './pages/overlay/ScrollRegionFrame';

// 根据 URL pathname 判断当前窗口类型
function Router() {
  const pathname = window.location.pathname;

  if (pathname.includes('pin.html')) {
    return <PinWindow />;
  }

  if (pathname.includes('overlay.html')) {
    return <FullscreenOverlay />;
  }

  if (pathname.includes('preview.html')) {
    return <PreviewWindow />;
  }

  if (pathname.includes('scroll_toolbar.html')) {
    return <ScrollCaptureToolbar />;
  }

  if (pathname.includes('scroll_frame.html')) {
    return <ScrollRegionFrame />;
  }

  // 主窗口
  return <App />;
}

// 注意：不启用 StrictMode —— 桌面多窗口 + 一次性 invoke payload 的场景下，
// 双挂载会导致动态窗口（overlay/preview）二次取 payload 拿到 None 而自动关闭。
ReactDOM.createRoot(document.getElementById('root')!).render(
  <Router />,
);
