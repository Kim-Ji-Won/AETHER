import { StrictMode, useEffect, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { invoke } from '@tauri-apps/api/core';
import './styles.css';
import { Shell } from './Shell';

function App() {
  const [lit, setLit] = useState(false);
  const [version, setVersion] = useState<number | null>(null);
  const [checking, setChecking] = useState(false);
  const [versionFailed, setVersionFailed] = useState(false);
  const [trustStatus, setTrustStatus] = useState<'unchecked' | 'checking' | 'trusted' | 'untrusted' | 'failed'>('unchecked');

  const [counter, setCounter] = useState<number | null>(null);
  const [counterPending, setCounterPending] = useState(false);
  const [counterFailed, setCounterFailed] = useState(false);

  useEffect(() => {
    invoke<number>('get_experiment_counter').then(setCounter).catch(() => setCounterFailed(true));
  }, []);

  async function requestIncrement() {
    setCounterPending(true);
    setCounterFailed(false);
    try {
      setCounter(await invoke<number>('request_counter_increment'));
    } catch {
      setCounterFailed(true);
    } finally {
      setCounterPending(false);
    }
  }

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

  return <Shell lit={lit} onLight={() => setLit(true)} nativeChecks={<div className="native-checks">
          <div className="native-version">
            <button type="button" onClick={requestIncrement} disabled={counterPending}>
              실험 카운터 +1 요청
            </button>
            <p role="status" aria-live="polite">
              실험 카운터: {counter ?? '확인 전'}
              {counterPending ? ' · 승인 대기 중' : counterFailed ? ' · 요청을 처리하지 못했어요' : ''}
            </p>
          </div>
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
        </div>} />;
}

createRoot(document.getElementById('root')!).render(<StrictMode><App /></StrictMode>);
