import { StrictMode, useState } from 'react';
import { createRoot } from 'react-dom/client';
import './styles.css';

function App() {
  const [lit, setLit] = useState(false);

  return (
    <main className={lit ? 'app is-lit' : 'app'}>
      <header className="topbar">
        <span className="signature">A / E</span>
        <span className="edition">FIRST CONTACT <span>—</span> 001</span>
      </header>
      <section className="core-section" aria-labelledby="wordmark">
        <div className="core" aria-hidden="true">
          <div className="orbit orbit-outer" />
          <div className="orbit orbit-inner" />
          <div className="core-ring" />
          <div className="core-center"><span /><span /><span /></div>
          <span className="axis axis-top" />
          <span className="axis axis-bottom" />
        </div>
        <p className="eyebrow">YOUR QUIET UNIVERSE</p>
        <h1 id="wordmark">AETHER</h1>
        <p className="status" role="status" aria-live="polite">
          <span className="status-dot" aria-hidden="true" />
          {lit ? '코어 점등됨' : '코어 대기 중'}
        </p>
        <p className="description">작은 빛으로 시작하는 새로운 연결.</p>
        <button type="button" onClick={() => setLit(true)} disabled={lit}>
          <span aria-hidden="true">{lit ? '✓' : '◈'}</span>
          {lit ? '점등 완료' : '코어 점등'}
        </button>
      </section>
      <footer><span>LOCAL EXPERIENCE</span><span>첫 번째 신호를 기다립니다.</span><span>P0 · E1a</span></footer>
    </main>
  );
}

createRoot(document.getElementById('root')!).render(<StrictMode><App /></StrictMode>);
