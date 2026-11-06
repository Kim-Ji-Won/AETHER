import { StrictMode, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import './styles.css';

function App() {
  const [lit, setLit] = useState(false);
  const [version, setVersion] = useState<number | null>(null);
  const [checking, setChecking] = useState(false);
  const [versionFailed, setVersionFailed] = useState(false);
  const [trustStatus, setTrustStatus] = useState<'unchecked' | 'checking' | 'trusted' | 'untrusted' | 'failed'>('unchecked');

  async function checkAccessibilityTrust() {
    setTrustStatus('checking');
    try {
      const trusted = await invoke<unknown>('get_accessibility_trust_status');
      if (typeof trusted !== 'boolean') throw new Error('Invalid trust status');
      setTrustStatus(trusted ? 'trusted' : 'untrusted');
    } catch {
      setTrustStatus('failed');
    }
  }

  async function checkVersion() {
    setChecking(true);
    setVersion(null);
    setVersionFailed(false);
    try {
      const major = await invoke<number>('get_macos_major_version');
      if (!Number.isInteger(major) || major <= 0) throw new Error('Invalid major version');
      setVersion(major);
    } catch {
      setVersionFailed(true);
    } finally {
      setChecking(false);
    }
  }

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
        <div className="native-checks">
          <div className="native-version">
            <button type="button" onClick={checkVersion} disabled={checking}>
              {checking ? '확인 중…' : 'macOS 버전 확인'}
            </button>
            <p role="status" aria-live="polite">
              {versionFailed ? '버전을 확인하지 못했어요' : version !== null ? `macOS ${version}` : ''}
            </p>
          </div>
          <div className="native-version">
            <button type="button" onClick={checkAccessibilityTrust} disabled={trustStatus === 'checking'}>
              손쉬운 사용 상태 확인
            </button>
            <p role="status" aria-live="polite">
              {trustStatus === 'unchecked' ? '아직 확인하지 않았어요'
                : trustStatus === 'checking' ? '확인 중…'
                : trustStatus === 'trusted' ? '손쉬운 사용 권한이 있어요'
                : trustStatus === 'untrusted' ? '현재 손쉬운 사용 권한이 없어요'
                : '상태를 확인하지 못했어요'}
            </p>
          </div>
        </div>
      </section>
      <footer><span>LOCAL EXPERIENCE</span><span>첫 번째 신호를 기다립니다.</span><span>P0 · E1a</span></footer>
    </main>
  );
}

createRoot(document.getElementById('root')!).render(<StrictMode><App /></StrictMode>);
