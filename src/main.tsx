import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import PinWindow from './pages/pin/PinWindow';

// 根据 URL pathname 判断当前窗口类型
// 主窗口: / 或空
// 贴图窗口: /pin.html
function Router() {
  const pathname = window.location.pathname;

  if (pathname.includes('pin.html')) {
    return <PinWindow />;
  }

  // 主窗口
  return <App />;
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <Router />
  </React.StrictMode>,
);
